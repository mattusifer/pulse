use actix::prelude::*;
use systemstat::{Platform, System};

use super::{messages::ScheduleMessage, Service, ServiceId};
use crate::{
    broadcast::Broadcast,
    config::{Config, FilesystemConfig, SystemMonitorConfig},
    error::{Error, Result},
};

pub struct SystemMonitor {
    system: System,
    broadcast: Broadcast,
    config: SystemMonitorConfig,
}
impl SystemMonitor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            system: System::new(),
            broadcast: Broadcast::new()?,
            config: Config::from_file()?.system_monitor.unwrap(),
        })
    }

    /// Get the list of mounts from the config for this service
    pub fn filesystems(&self) -> &Vec<FilesystemConfig> {
        &self.config.filesystems
    }
}

impl Service for SystemMonitor {
    fn id() -> ServiceId {
        ServiceId::from("SystemMonitor")
    }
}

impl Actor for SystemMonitor {
    type Context = Context<Self>;
}

impl Handler<ScheduleMessage> for SystemMonitor {
    type Result = Result<()>;

    fn handle(&mut self, msg: ScheduleMessage, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ScheduleMessage::CheckDiskUsage => self
                .filesystems()
                .iter()
                .map(|filesystem_config| {
                    filesystem_config
                        .mount
                        .to_str()
                        .ok_or_else(|| Error::invalid_unicode_path(filesystem_config.mount.clone()))
                        .and_then(|path| self.system.mount_at(path).map_err(Into::into))
                        .and_then(|filesystem| {
                            let disk_usage = (filesystem.avail.as_usize() as f32
                                / filesystem.total.as_usize() as f32) * 100 as f32;
                            if disk_usage > filesystem_config.available_space_alert_above {
                                self.broadcast.email(
                                    "Filesystem Alert",
                                    format!(
                                    "Filesystem mounted at {} has {:.2}% disk usage, which is above the max of {:.2}",
                                    filesystem.fs_mounted_on,
                                    disk_usage,
                                    filesystem_config.available_space_alert_above
                                ),
                                )
                            } else {
                                Ok(())
                            }
                        })
                })
                .collect::<Result<Vec<_>>>()
                .map_err(|e| {
                    eprintln!("{}", e);
                    e
                })
                .map(|_| ()),
        }
    }
}
