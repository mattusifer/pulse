use actix::prelude::*;

use super::messages::ScheduleMessage;
use crate::config::{config, ScheduleConfig};
use crate::error::Error;

/// The scheduler is responsible for kicking off configured tasks at
/// the correct times by sending messages to other services that
/// actually perform those tasks
pub struct Scheduler {
    config: Vec<ScheduleConfig>,
    services: Vec<Recipient<ScheduleMessage>>,
}
impl Scheduler {
    pub fn new() -> Self {
        Self {
            config: config().schedules,
            services: vec![],
        }
    }

    /// Add a service to the scheduler
    pub fn add_service(&mut self, service: Recipient<ScheduleMessage>) {
        self.services.push(service)
    }
}

impl Actor for Scheduler {
    type Context = Context<Self>;

    /// When the scheduler is started, it will configure the actix
    /// context to send the correct messages to the configured
    /// services on the configured schedule
    fn started(&mut self, ctx: &mut Context<Self>) {
        for schedule in &self.config {
            for service in &self.services {
                let schedule = schedule.clone();
                let service = service.clone();
                ctx.run_interval(schedule.schedule_interval, move |_, _| {
                    let schedule = schedule.clone();
                    service.do_send(schedule.message).unwrap_or_else(|e| {
                        eprintln!("{}", Into::<Error>::into(e))
                    })
                });
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{
        sync::{Arc, Mutex},
        thread, time,
    };

    use crate::{
        config::{self, Config},
        error::Result,
    };

    struct TestActor {
        pub messages_recieved: Arc<Mutex<Vec<ScheduleMessage>>>,
    }
    impl Actor for TestActor {
        type Context = Context<Self>;
    }

    impl Handler<ScheduleMessage> for TestActor {
        type Result = Result<()>;

        fn handle(
            &mut self,
            msg: ScheduleMessage,
            _ctx: &mut Context<Self>,
        ) -> Self::Result {
            self.messages_recieved.lock().unwrap().push(msg);
            Ok(())
        }
    }

    #[test]
    fn scheduler_sends_messages_to_configured_service() {
        let mut test_config = Config::default();
        test_config.schedules.push(ScheduleConfig {
            schedule_interval: time::Duration::from_millis(25),
            message: ScheduleMessage::CheckDiskUsage,
        });

        let system = System::new("test");

        let messages_received: Arc<Mutex<Vec<ScheduleMessage>>> =
            Arc::new(Mutex::new(vec![]));

        let test_actor = TestActor {
            messages_recieved: Arc::clone(&messages_received),
        };
        let addr = test_actor.start();
        let recipient = Addr::recipient(addr);

        run_with_config!(test_config, {
            let mut scheduler = Scheduler::new();
            scheduler.add_service(recipient);
            scheduler.start();

            let current = System::current();
            thread::spawn(move || {
                thread::sleep(time::Duration::from_millis(300));

                let messages = messages_received.lock().unwrap().clone();

                assert!(messages.len() > 2);
                assert!(messages
                    .into_iter()
                    .all(|msg| msg == ScheduleMessage::CheckDiskUsage));

                current.stop();
            });

            system.run();
        });
    }

    #[test]
    fn scheduler_doesnt_send_to_unconfigured_service() {
        let mut test_config = Config::default();
        test_config.schedules.push(ScheduleConfig {
            schedule_interval: time::Duration::from_millis(25),
            message: ScheduleMessage::CheckDiskUsage,
        });

        let system = System::new("test");

        let messages_received: Arc<Mutex<Vec<ScheduleMessage>>> =
            Arc::new(Mutex::new(vec![]));

        TestActor {
            messages_recieved: Arc::clone(&messages_received),
        }
        .start();

        run_with_config!(test_config, {
            let scheduler = Scheduler::new();
            scheduler.start();

            let current = System::current();
            thread::spawn(move || {
                thread::sleep(time::Duration::from_millis(60));
                assert!(messages_received.lock().unwrap().is_empty());
                current.stop();
            });

            system.run();
        });
    }
}
