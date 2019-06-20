use std::time::Duration;

use actix::prelude::*;
use systemstat::{Filesystem, Platform, System as LocalSystem};

use super::{
    broadcast::OUTBOX,
    messages::{BroadcastEvent, ScheduledStreamMessage},
};

use crate::{
    config::{
        config, FilesystemConfig, ScheduledStreamConfig, SystemMonitorConfig,
    },
    db::{database, models},
    error::{Error, Result},
};

pub struct SystemMonitor {
    system: LocalSystem,
    config: SystemMonitorConfig,
    streams: Vec<ScheduledStreamConfig>,
}
impl SystemMonitor {
    pub fn new() -> Self {
        Self {
            system: LocalSystem::new(),
            config: config().system_monitor.unwrap(),
            streams: config().streams,
        }
    }

    /// Get the list of mounts from the config for this service
    fn filesystems(&self) -> &Vec<FilesystemConfig> {
        &self.config.filesystems
    }

    fn get_mount(
        &self,
        filesystem_config: &FilesystemConfig,
    ) -> Result<Filesystem> {
        filesystem_config
            .mount
            .to_str()
            .ok_or_else(|| {
                Error::invalid_unicode_path(filesystem_config.mount.clone())
            })
            .and_then(|path| self.system.mount_at(path).map_err(Into::into))
    }

    fn check_all_filesystems_usage(&self) -> Result<()> {
        self.filesystems()
            .iter()
            .map(|fs| self.check_filesystem_usage(fs))
            .collect::<Result<Vec<_>>>()
            .map(|_| ())
    }

    fn check_filesystem_usage(
        &self,
        filesystem_config: &FilesystemConfig,
    ) -> Result<()> {
        self.get_mount(filesystem_config)
            .and_then(|filesystem| {
                let space_used =
                    filesystem.total.as_u64() - filesystem.avail.as_u64();

                database()
                    .insert_disk_usage(models::NewDiskUsage::new(
                        filesystem.fs_mounted_on.clone(),
                        space_used as i64,
                        filesystem.total.as_u64() as i64,
                    ))
                    .map(|_| (filesystem, space_used))
            })
            .and_then(|(filesystem, space_used)| {
                let disk_usage = (space_used as f64
                    / filesystem.total.as_u64() as f64)
                    * 100 as f64;

                if disk_usage > filesystem_config.available_space_alert_above {
                    let message = BroadcastEvent::HighDiskUsage {
                        filesystem_mount: filesystem.fs_mounted_on,
                        current_usage: disk_usage,
                        max_usage: filesystem_config
                            .available_space_alert_above,
                    };

                    OUTBOX.push(message)?;
                }

                Ok(())
            })
    }
}

impl Actor for SystemMonitor {
    type Context = Context<Self>;

    /// When the system monitor is started, begin continually running
    /// the configured tasks
    fn started(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(Duration::from_millis(250), move |this, _ctx| {
            for stream in &this.streams {
                match stream.message {
                    ScheduledStreamMessage::CheckDiskUsage => {
                        this.check_all_filesystems_usage().or_else::<Error, _>(|e| {
                            log::error!("Error encountered checking filesystem usage: {:?}", e);
                            Ok(())
                        }).unwrap()
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        config::{Config, FilesystemConfig},
        services::messages::BroadcastEventType,
    };
    use std::{thread, time::Duration};

    #[test]
    fn system_monitor_checks_disk_usage() {
        let mut config = Config::default();
        config.system_monitor = Some(SystemMonitorConfig {
            filesystems: vec![FilesystemConfig {
                mount: "/".into(),
                available_space_alert_above: 0.0,
            }],
        });
        config.streams = vec![ScheduledStreamConfig {
            message: ScheduledStreamMessage::CheckDiskUsage,
        }];

        let system = System::new("test");

        run_with_config!(config, {
            SystemMonitor::new().start();
        });

        run_with_outbox!({
            let current = System::current();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(2000));

                let outgoing_message = OUTBOX.pop().unwrap();
                assert_eq!(
                    outgoing_message.event_type(),
                    BroadcastEventType::HighDiskUsage
                );

                current.stop()
            });
        });

        let state = run_with_db!(system);
        assert!(state.disk_usage.len() > 2);
    }
}
