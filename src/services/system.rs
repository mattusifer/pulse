use actix::prelude::*;
use chrono::Duration;
use systemstat::{Filesystem, Platform, System as LocalSystem};

use super::{messages::ScheduleMessage, Service, ServiceId};

use crate::{
    broadcast::{Broadcast, BroadcastMessage},
    config::{Config, FilesystemConfig, SystemMonitorConfig},
    error::{Error, Result},
};

pub struct SystemMonitor {
    system: LocalSystem,
    broadcast: Broadcast,
    config: SystemMonitorConfig,
}
impl SystemMonitor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            system: LocalSystem::new(),
            broadcast: Broadcast::new()?,
            config: Config::from_file()?.system_monitor.unwrap(),
        })
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
            let identifier =
                serde_json::to_string(&ScheduleMessage::CheckDiskUsage)?;

            if disk_usage > filesystem_config.available_space_alert_above {
                broadcast!(
                    Duration::minutes(1),
                    identifier,
                    vec![BroadcastMessage::Email {
                        subject: "Filesystem Alert".into(),
                        body: format!(
                            "Filesystem mounted at {} has {:.2}% disk usage, \
                             which is above the max of {:.2}",
                            filesystem.fs_mounted_on,
                            disk_usage,
                            filesystem_config.available_space_alert_above
                        )
                    }],
                    |msg| self.broadcast.broadcast(msg)
                )
            } else {
                Ok(())
            }
        })
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
                .map_err(|e| {
                    eprintln!("{}", e);
                    e
                })
                .map(|_| ()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db;

    #[test]
    fn check_disk_usage_sets_db_once() {
        System::run(move || {
            let addr = SystemMonitor::create(move |_ctx| {
                SystemMonitor::new().unwrap()
            });
            let recipient = Addr::recipient(addr);

            recipient.do_send(ScheduleMessage::CheckDiskUsage).unwrap();

            let initial_date = db::get_date(format!(
                "broadcast_{}",
                serde_json::to_string(&ScheduleMessage::CheckDiskUsage)
                    .unwrap()
            ))
            .unwrap()
            .unwrap();

            recipient.do_send(ScheduleMessage::CheckDiskUsage).unwrap();

            let new_date = db::get_date(format!(
                "broadcast_{}",
                serde_json::to_string(&ScheduleMessage::CheckDiskUsage)
                    .unwrap()
            ))
            .unwrap()
            .unwrap();

            assert_eq!(initial_date, new_date);

            System::current().stop();
        });
    }
}
