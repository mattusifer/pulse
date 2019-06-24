use std::{collections::HashMap, time::Duration};

use actix::{Actor, AsyncContext, Context, Handler, Message, Recipient};
use systemstat::{Filesystem, Platform, System as LocalSystem};

use crate::{
    config::{
        config, FilesystemConfig, ScheduledStreamConfig, SystemMonitorConfig,
    },
    db::{database, models},
    error::{Error, Result},
    services::{
        broadcast::{BroadcastEvent, OUTBOX},
        scheduler::ScheduledStreamMessage,
    },
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
    subscribers: HashMap<usize, Subscriber>,
    ports: Box<SystemMonitorPorts>,
}
impl SystemMonitor {
    pub fn new() -> Self {
        Self {
            system: LocalSystem::new(),
            config: config().system_monitor.unwrap(),
            streams: config().streams,
            subscribers: HashMap::new(),
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
            subscribers: HashMap::new(),
            ports,
        }
    }

    /// Get the list of mounts from the config for this service
    fn filesystems(&self) -> &Vec<FilesystemConfig> {
        &self.config.filesystems
    }

    fn next_subscriber_id(&self) -> usize {
        let id: usize = rand::random();
        if self.subscribers.contains_key(&id) {
            self.next_subscriber_id()
        } else {
            id
        }
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
                let disk_usage = models::NewDiskUsage::new(
                    filesystem.fs_mounted_on.clone(),
                    disk_usage,
                );

                // record current usage in the database
                self.ports
                    .record_disk_usage(disk_usage.clone())
                    .map(|disk_usage| (filesystem, disk_usage))
            })
            .and_then(|(filesystem, disk_usage)| {
                // send filesystem updates to all subscribers
                self.subscribers
                    .values()
                    .map(|subscriber| {
                        subscriber
                            .do_send(disk_usage.clone())
                            .map_err(Into::into)
                    })
                    .collect::<Result<Vec<_>>>()
                    .map(|_| (filesystem, disk_usage))
            })
            .and_then(|(filesystem, disk_usage)| {
                // if the current usage exceeds the threshold, send an alert
                if disk_usage.percent_disk_used
                    > filesystem_config.available_space_alert_above
                {
                    let message = BroadcastEvent::HighDiskUsage {
                        filesystem_mount: filesystem.fs_mounted_on,
                        current_usage: disk_usage.percent_disk_used,
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

/// Subscribe to system updates
type Subscriber = Recipient<models::DiskUsage>;

pub struct Subscribe(pub Subscriber);
impl Message for Subscribe {
    type Result = usize;
}

#[derive(Message)]
pub struct Unsubscribe(pub usize);

impl Handler<Subscribe> for SystemMonitor {
    type Result = usize;

    fn handle(
        &mut self,
        msg: Subscribe,
        _: &mut Self::Context,
    ) -> Self::Result {
        let id = self.next_subscriber_id();
        self.subscribers.insert(id, msg.0);
        id
    }
}

impl Handler<Unsubscribe> for SystemMonitor {
    type Result = ();

    fn handle(&mut self, msg: Unsubscribe, _: &mut Self::Context) {
        self.subscribers.remove(&msg.0);
    }
}

#[cfg(test)]
mod test {
    use std::{
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };

    use actix::{Addr, System};
    use futures::{future, Future};
    use tokio_timer::Delay;

    use super::*;
    use crate::{
        config::FilesystemConfig, services::broadcast::BroadcastEventType,
    };

    struct GetState;
    impl Message for GetState {
        type Result = String;
    }

    struct TestSubscriber {
        updates: Vec<models::DiskUsage>,
    }
    impl TestSubscriber {
        pub fn new() -> Self {
            Self { updates: vec![] }
        }
    }
    impl Actor for TestSubscriber {
        type Context = Context<Self>;
    }
    impl Handler<models::DiskUsage> for TestSubscriber {
        type Result = ();

        fn handle(&mut self, update: models::DiskUsage, _: &mut Self::Context) {
            self.updates.push(update)
        }
    }
    impl Handler<GetState> for TestSubscriber {
        type Result = String;

        fn handle(
            &mut self,
            _: GetState,
            _: &mut Self::Context,
        ) -> Self::Result {
            serde_json::to_string(&self.updates).unwrap()
        }
    }

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
                recorded_at: chrono::NaiveDateTime::from_timestamp(0, 0),
            })
        }

        fn send_alert(&self, event: BroadcastEvent) -> Result<()> {
            self.lock().unwrap().sent_alerts.push(event);
            Ok(())
        }
    }

    fn test_monitor(
        ports: Arc<Mutex<TestSystemMonitorPorts>>,
    ) -> SystemMonitor {
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
            Box::new(ports),
        )
    }

    #[test]
    fn system_monitor_records_disk_usage() {
        System::run(|| {
            let ports = Arc::new(Mutex::new(TestSystemMonitorPorts::new()));
            test_monitor(Arc::clone(&ports)).start();

            actix_rt::spawn(futures::lazy(move || {
                Delay::new(Instant::now() + Duration::from_millis(30)).then(
                    move |_| {
                        let ports = ports.lock().unwrap();
                        assert!(ports.recorded_disk_usage.len() == 3);

                        System::current().stop();
                        future::result(Ok(()))
                    },
                )
            }))
        })
        .unwrap()
    }

    #[test]
    fn system_monitor_sends_alerts() {
        System::run(|| {
            let ports = Arc::new(Mutex::new(TestSystemMonitorPorts::new()));
            test_monitor(Arc::clone(&ports)).start();

            actix_rt::spawn(futures::lazy(move || {
                Delay::new(Instant::now() + Duration::from_millis(30)).then(
                    move |_| {
                        let ports = ports.lock().unwrap();
                        assert!(ports.sent_alerts.len() == 3);
                        assert_eq!(
                            ports.sent_alerts[0].event_type(),
                            BroadcastEventType::HighDiskUsage
                        );

                        System::current().stop();
                        future::result(Ok(()))
                    },
                )
            }))
        })
        .unwrap()
    }

    #[test]
    fn system_monitor_sends_updates_to_subscribers() {
        System::run(|| {
            let monitor = test_monitor(Arc::new(Mutex::new(
                TestSystemMonitorPorts::new(),
            )))
            .start();
            let subscriber = TestSubscriber::new().start();

            monitor.do_send(Subscribe(Addr::recipient(subscriber.clone())));

            actix_rt::spawn(futures::lazy(move || {
                Delay::new(Instant::now() + Duration::from_millis(30))
                    .then(move |_| subscriber.send(GetState).map_err(|_| ()))
                    .map(|msg| {
                        let updates: Vec<models::DiskUsage> =
                            serde_json::from_str(&msg).unwrap();
                        println!("{:?}", updates);
                        assert!(updates.len() == 3);

                        System::current().stop();
                    })
            }))
        })
        .unwrap()
    }
}
