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

trait SystemMonitorPorts {
    fn record_disk_usage(
        &self,
        disk_usage: models::NewDiskUsage,
    ) -> Result<models::DiskUsage>;

    fn send_alert(&self, event: BroadcastEvent) -> Result<()>;
}

struct LiveSystemMonitorPorts;
impl SystemMonitorPorts for LiveSystemMonitorPorts {
    fn record_disk_usage(
        &self,
        disk_usage: models::NewDiskUsage,
    ) -> Result<models::DiskUsage> {
        database().insert_disk_usage(disk_usage)
    }

    fn send_alert(&self, event: BroadcastEvent) -> Result<()> {
        OUTBOX.push(event).map_err(Into::into)
    }
}

pub struct SystemMonitor {
    system: LocalSystem,
    config: SystemMonitorConfig,
    streams: Vec<ScheduledStreamConfig>,
    ports: Box<SystemMonitorPorts>,
}
impl SystemMonitor {
    pub fn new() -> Self {
        Self {
            system: LocalSystem::new(),
            config: config().system_monitor.unwrap(),
            streams: config().streams,
            ports: Box::new(LiveSystemMonitorPorts),
        }
    }

    #[cfg(test)]
    fn test(
        config: SystemMonitorConfig,
        streams: Vec<ScheduledStreamConfig>,
        ports: Box<SystemMonitorPorts>,
    ) -> Self {
        Self {
            system: LocalSystem::new(),
            config,
            streams,
            ports,
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
                let disk_usage = ((filesystem.total.as_u64()
                    - filesystem.avail.as_u64())
                    as f64
                    / filesystem.total.as_u64() as f64)
                    * 100_f64;

                self.ports
                    .record_disk_usage(models::NewDiskUsage::new(
                        filesystem.fs_mounted_on.clone(),
                        disk_usage,
                    ))
                    .map(|_| (filesystem, disk_usage))
            })
            .and_then(|(filesystem, disk_usage)| {
                if disk_usage > filesystem_config.available_space_alert_above {
                    let message = BroadcastEvent::HighDiskUsage {
                        filesystem_mount: filesystem.fs_mounted_on,
                        current_usage: disk_usage,
                        max_usage: filesystem_config
                            .available_space_alert_above,
                    };

                    self.ports.send_alert(message)?
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
        ctx.run_interval(Duration::from_millis(self.config.tick_ms), move |this, _ctx| {
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
    use diesel::data_types::PgTimestamp;
    use std::{
        sync::{Arc, Mutex},
        thread,
        time::Duration,
    };

    use crate::{
        config::FilesystemConfig, services::messages::BroadcastEventType,
    };

    struct TestSystemMonitorPorts {
        recorded_disk_usage: Vec<models::NewDiskUsage>,
        sent_alerts: Vec<BroadcastEvent>,
    }
    impl TestSystemMonitorPorts {
        pub fn new() -> Self {
            Self {
                recorded_disk_usage: vec![],
                sent_alerts: vec![],
            }
        }
    }
    impl SystemMonitorPorts for Arc<Mutex<TestSystemMonitorPorts>> {
        fn record_disk_usage(
            &self,
            disk_usage: models::NewDiskUsage,
        ) -> Result<models::DiskUsage> {
            self.lock()
                .unwrap()
                .recorded_disk_usage
                .push(disk_usage.clone());
            Ok(models::DiskUsage {
                id: 0,
                mount: disk_usage.mount,
                percent_disk_used: disk_usage.percent_disk_used,
                recorded_at: PgTimestamp(0),
            })
        }

        fn send_alert(&self, event: BroadcastEvent) -> Result<()> {
            self.lock().unwrap().sent_alerts.push(event);
            Ok(())
        }
    }

    #[test]
    fn system_monitor_checks_disk_usage() {
        let system = System::new("test");

        let ports = Arc::new(Mutex::new(TestSystemMonitorPorts::new()));

        SystemMonitor::test(
            SystemMonitorConfig {
                filesystems: vec![FilesystemConfig {
                    mount: "/".into(),
                    available_space_alert_above: 0.0,
                }],
                tick_ms: 10,
            },
            vec![ScheduledStreamConfig {
                message: ScheduledStreamMessage::CheckDiskUsage,
            }],
            Box::new(Arc::clone(&ports)),
        )
        .start();

        let current = System::current();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));
            current.stop()
        });

        system.run().unwrap();

        let ports = ports.lock().unwrap();
        assert!(ports.recorded_disk_usage.len() >= 2);
        assert!(ports.sent_alerts.len() >= 2);
        assert_eq!(
            ports.sent_alerts[0].event_type(),
            BroadcastEventType::HighDiskUsage
        );
    }
}
