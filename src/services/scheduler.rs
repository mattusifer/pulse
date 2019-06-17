use std::str::FromStr;
use std::time::Duration;

use actix::prelude::*;
use chrono::Local;
use cron::Schedule as CronSchedule;

use super::messages::ScheduleMessage;
use crate::config::{config, SchedulerConfig};
use crate::db::{database, models};
use crate::error::Error;

/// The scheduler is responsible for kicking off configured tasks at
/// the correct times by sending messages to other services that
/// actually perform those tasks
pub struct Scheduler {
    config: SchedulerConfig,
    services: Vec<Recipient<ScheduleMessage>>,
}
impl Scheduler {
    pub fn new() -> Self {
        Self {
            config: config().scheduler,
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
        let tick = self.config.tick_ms;
        let chrono_tick = chrono::Duration::milliseconds(tick.into());
        let std_tick = Duration::from_millis(tick.into());

        for schedule in &self.config.schedules {
            for service in &self.services {
                let schedule = schedule.clone();
                let service = service.clone();

                // TODO: validate the schedule string before it gets here
                let cron_schedule = schedule
                    .cron
                    .clone()
                    .and_then(|c| CronSchedule::from_str(&c).ok());

                ctx.run_interval(std_tick, move |_, _| {
                    if let Some(ref cron_schedule) = cron_schedule {
                        let next_date =
                            cron_schedule.upcoming(Local).next().unwrap();
                        let one_tick_before = next_date - chrono_tick;
                        let now = Local::now();

                        if !(one_tick_before <= now && now <= next_date) {
                            return;
                        }
                    }

                    // insert the message into the DB
                    serde_json::to_string(&schedule.message)
                        .map_err(Into::into)
                        .and_then(|m| {
                            database()
                                .insert_message(models::NewMessage::new(m))
                                .map(|_| ())
                        })
                        .unwrap_or_else(|e| {
                            eprintln!("{}", Into::<Error>::into(e))
                        });

                    service.do_send(schedule.clone().message).unwrap_or_else(
                        |e| eprintln!("{}", Into::<Error>::into(e)),
                    )
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
        config::{Config, ScheduleConfig},
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
    fn scheduler_sends_messages_to_configured_service_on_cron_schedule() {
        let mut test_config = Config::default();
        test_config.scheduler.tick_ms = 1000;
        test_config.scheduler.schedules.push(ScheduleConfig {
            cron: Some("* * * * * * *".to_string()),
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
        });

        let current = System::current();
        thread::spawn(move || {
            thread::sleep(time::Duration::from_millis(3000));

            let messages = messages_received.lock().unwrap().clone();

            assert!(messages.len() == 2);
            assert!(messages
                .into_iter()
                .all(|msg| msg == ScheduleMessage::CheckDiskUsage));

            current.stop();
        });

        run_with_db!(system);
    }

    #[test]
    fn scheduler_sends_messages_to_configured_service() {
        let mut test_config = Config::default();
        test_config.scheduler.tick_ms = 25;
        test_config.scheduler.schedules.push(ScheduleConfig {
            cron: None,
            message: ScheduleMessage::CheckDiskUsage,
        });

        let system = System::new("test");

        let messages_received: Arc<Mutex<Vec<ScheduleMessage>>> =
            Arc::new(Mutex::new(vec![]));
        let messages_received_clone = Arc::clone(&messages_received);

        let test_actor = TestActor {
            messages_recieved: Arc::clone(&messages_received),
        };
        let addr = test_actor.start();
        let recipient = Addr::recipient(addr);

        run_with_config!(test_config, {
            let mut scheduler = Scheduler::new();
            scheduler.add_service(recipient);
            scheduler.start();
        });

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

        let db_state = run_with_db!(system);

        // messages should have been inserted into the database
        assert!(
            db_state.messages.len()
                == messages_received_clone.lock().unwrap().clone().len()
        );
    }

    #[test]
    fn scheduler_doesnt_send_to_unconfigured_service() {
        let mut test_config = Config::default();
        test_config.scheduler.tick_ms = 25;
        test_config.scheduler.schedules.push(ScheduleConfig {
            cron: None,
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
            Scheduler::new().start();
        });

        let current = System::current();
        thread::spawn(move || {
            thread::sleep(time::Duration::from_millis(60));
            assert!(messages_received.lock().unwrap().is_empty());
            current.stop();
        });

        run_with_db!(system);
    }
}
