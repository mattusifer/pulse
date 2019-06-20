use actix::prelude::*;

use super::messages::ScheduledTaskMessage;
use crate::config::{config, ScheduledTaskConfig};
use crate::db::{database, models};
use crate::error::Error;

/// The scheduler is responsible for kicking off configured tasks at
/// the correct times by sending messages to other services that
/// actually perform those tasks
pub struct Scheduler {
    tasks: Vec<ScheduledTaskConfig>,
    task_runners: Vec<Recipient<ScheduledTaskMessage>>,
}
impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: config().tasks,
            task_runners: vec![],
        }
    }

    /// Add a service to the scheduler
    pub fn add_task_runner(
        &mut self,
        task_runner: Recipient<ScheduledTaskMessage>,
    ) {
        self.task_runners.push(task_runner)
    }

    fn schedule_task(
        &self,
        ctx: &mut Context<Self>,
        task: ScheduledTaskConfig,
    ) -> () {
        // record this message in the db
        serde_json::to_string(&task.message)
            .map_err(Into::into)
            .and_then(|t| {
                database().insert_task(models::NewTask::new(t)).map(|_| ())
            })
            .unwrap_or_else(|e| eprintln!("{}", Into::<Error>::into(e)));

        // send this message to configured task_runners
        for runner in &self.task_runners {
            runner
                .do_send(task.clone().message)
                .unwrap_or_else(|e| eprintln!("{}", Into::<Error>::into(e)))
        }

        // schedule the next run of this task based on its cron schedule
        ctx.run_later(task.duration_until_next(), |this, ctx| {
            this.schedule_task(ctx, task)
        });
    }
}

impl Actor for Scheduler {
    type Context = Context<Self>;

    /// When the scheduler is started, it will configure the actix
    /// context to send the configured messages to task_runners on the
    /// configured schedule
    fn started(&mut self, ctx: &mut Context<Self>) {
        // start tasks
        for task in &self.tasks {
            let task = task.clone();
            ctx.run_later(task.duration_until_next(), move |this, ctx| {
                this.schedule_task(ctx, task.clone())
            });
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
        config::{Config, ScheduledTaskConfig},
        error::Result,
    };

    struct TestActor {
        pub messages_recieved: Arc<Mutex<Vec<ScheduledTaskMessage>>>,
    }
    impl Actor for TestActor {
        type Context = Context<Self>;
    }

    impl Handler<ScheduledTaskMessage> for TestActor {
        type Result = Result<()>;

        fn handle(
            &mut self,
            msg: ScheduledTaskMessage,
            _ctx: &mut Context<Self>,
        ) -> Self::Result {
            self.messages_recieved.lock().unwrap().push(msg);
            Ok(())
        }
    }

    #[test]
    fn scheduler_sends_messages_to_configured_service_on_cron_schedule() {
        let mut test_config = Config::default();
        test_config.tasks.push(ScheduledTaskConfig {
            cron: "* * * * * * *".to_string(),
            message: ScheduledTaskMessage::FetchNews,
        });

        let system = System::new("test");

        let messages_received: Arc<Mutex<Vec<ScheduledTaskMessage>>> =
            Arc::new(Mutex::new(vec![]));

        let test_actor = TestActor {
            messages_recieved: Arc::clone(&messages_received),
        };
        let addr = test_actor.start();
        let recipient = Addr::recipient(addr);

        run_with_config!(test_config, {
            let mut scheduler = Scheduler::new();
            scheduler.add_task_runner(recipient);
            scheduler.start();
        });

        let current = System::current();
        thread::spawn(move || {
            thread::sleep(time::Duration::from_millis(3750));

            let messages = messages_received.lock().unwrap().clone();

            assert!(messages.len() == 3);
            assert!(messages
                .into_iter()
                .all(|msg| msg == ScheduledTaskMessage::FetchNews));

            current.stop();
        });

        let state = run_with_db!(system);
        assert!(state.tasks.len() == 3);
    }

    #[test]
    fn scheduler_doesnt_send_to_unconfigured_service() {
        let mut test_config = Config::default();
        test_config.tasks.push(ScheduledTaskConfig {
            cron: "* * * * * * *".to_string(),
            message: ScheduledTaskMessage::FetchNews,
        });

        let system = System::new("test");

        let messages_received: Arc<Mutex<Vec<ScheduledTaskMessage>>> =
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
