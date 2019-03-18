use actix::prelude::*;
use systemstat::{Filesystem, Platform, System as LocalSystem};

use super::{
    broadcast::OUTBOX,
    messages::{BroadcastEvent, ScheduleMessage},
};

use crate::{
    config::{config, FilesystemConfig, SystemMonitorConfig},
    error::{Error, Result},
};

pub struct SystemMonitor {
    system: LocalSystem,
    config: SystemMonitorConfig,
}
impl SystemMonitor {
    pub fn new() -> Self {
        let config = config().system_monitor.unwrap();
        Self {
            system: LocalSystem::new(),
            config: config,
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

    fn check_disk_usage(
        &self,
        filesystem_config: &FilesystemConfig,
    ) -> Result<()> {
        self.get_mount(filesystem_config).and_then(|filesystem| {
            let disk_usage = (filesystem.avail.as_usize() as f32
                / filesystem.total.as_usize() as f32)
                * 100 as f32;

            if disk_usage > filesystem_config.available_space_alert_above {
                let message = BroadcastEvent::HighDiskUsage {
                    filesystem_mount: filesystem.fs_mounted_on,
                    current_usage: disk_usage,
                    max_usage: filesystem_config.available_space_alert_above,
                };

                OUTBOX.push(message)?;
            }

            Ok(())
        })
    }
}

impl Actor for SystemMonitor {
    type Context = Context<Self>;
}

impl Handler<ScheduleMessage> for SystemMonitor {
    type Result = Result<()>;

    fn handle(
        &mut self,
        msg: ScheduleMessage,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        match msg {
            ScheduleMessage::CheckDiskUsage => self
                .filesystems()
                .iter()
                .map(|fs| self.check_disk_usage(fs))
                .collect::<Result<Vec<_>>>()
                .map(|_| ()),
            _ => Ok(()),
        }
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

        let system = System::new("test");

        run_with_config!(config, {
            let addr = SystemMonitor::new().start();
            let recipient = Addr::recipient(addr);

            recipient.do_send(ScheduleMessage::CheckDiskUsage).unwrap();
        });

        let current = System::current();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));

            let outgoing_message = OUTBOX.pop().unwrap();
            assert_eq!(
                outgoing_message.event_type(),
                BroadcastEventType::HighDiskUsage
            );

            current.stop()
        });

        system.run();
    }
}
