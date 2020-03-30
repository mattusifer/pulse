mod messages;
pub use messages::*;

use actix::prelude::*;

use crate::{
    config::{config, ScheduledTaskConfig},
    db::{database, models},
    error::{Error, Result},
};

trait SchedulerPorts {
    fn insert_task(&self, task: models::NewTask) -> Result<()>;
}

struct LiveSchedulerPorts;
impl SchedulerPorts for LiveSchedulerPorts {
    fn insert_task(&self, task: models::NewTask) -> Result<()> {
        database().insert_task(task).map(|_| ())
    }
}

/// The scheduler is responsible for kicking off configured tasks at
/// the correct times by sending messages to other services that
/// actually perform those tasks
pub struct Scheduler {
    tasks: Vec<ScheduledTaskConfig>,
    task_runners: Vec<Recipient<ScheduledTaskMessage>>,
    ports: Box<dyn SchedulerPorts>,
}
impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: config().tasks,
            task_runners: vec![],
            ports: Box::new(LiveSchedulerPorts),
        }
    }

    #[cfg(test)]
    fn test(tasks: Vec<ScheduledTaskConfig>, test_ports: Box<dyn SchedulerPorts>) -> Self {
        Self {
            tasks,
            task_runners: vec![],
            ports: test_ports,
        }
    }

    /// Add a service to the scheduler
    pub fn add_task_runner(&mut self, task_runner: Recipient<ScheduledTaskMessage>) {
        self.task_runners.push(task_runner)
    }

    fn schedule_task(&self, ctx: &mut Context<Self>, task: ScheduledTaskConfig) {
        // record this message in the db
        serde_json::to_string(&task.message)
            .map_err(Into::into)
            .and_then(|t| self.ports.insert_task(models::NewTask::new(t)))
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

    use crate::{config::ScheduledTaskConfig, error::Result};

    struct TestActor {
        pub messages_recieved: Arc<Mutex<Vec<ScheduledTaskMessage>>>,
    }
    impl Actor for TestActor {
        type Context = Context<Self>;
    }

    impl Handler<ScheduledTaskMessage> for TestActor {
        type Result = Result<()>;

        fn handle(&mut self, msg: ScheduledTaskMessage, _ctx: &mut Context<Self>) -> Self::Result {
            self.messages_recieved.lock().unwrap().push(msg);
            Ok(())
        }
    }

    struct TestSchedulerPorts {
        inserted_tasks: Vec<models::NewTask>,
    }
    impl TestSchedulerPorts {
        pub fn new() -> Self {
            Self {
                inserted_tasks: vec![],
            }
        }
    }
    impl SchedulerPorts for Arc<Mutex<TestSchedulerPorts>> {
        fn insert_task(&self, task: models::NewTask) -> Result<()> {
            self.lock().unwrap().inserted_tasks.push(task);
            Ok(())
        }
    }

    #[test]
    fn scheduler_sends_messages_to_configured_service_on_cron_schedule() {
        let system = System::new("test");

        let messages_received: Arc<Mutex<Vec<ScheduledTaskMessage>>> = Arc::new(Mutex::new(vec![]));

        let test_actor = TestActor {
            messages_recieved: Arc::clone(&messages_received),
        };
        let addr = test_actor.start();
        let recipient = Addr::recipient(addr);

        let ports = Arc::new(Mutex::new(TestSchedulerPorts::new()));

        let mut scheduler = Scheduler::test(
            vec![ScheduledTaskConfig {
                cron: "* * * * * * *".to_string(),
                message: ScheduledTaskMessage::FetchNews,
            }],
            Box::new(Arc::clone(&ports)),
        );
        scheduler.add_task_runner(recipient);
        scheduler.start();

        let current = System::current();
        thread::spawn(move || {
            thread::sleep(time::Duration::from_millis(2500));

            let messages = messages_received.lock().unwrap().clone();

            assert!(messages.len() >= 2);
            assert!(messages
                .into_iter()
                .all(|msg| msg == ScheduledTaskMessage::FetchNews));

            current.stop();
        });

        system.run().unwrap();
        assert!(ports.lock().unwrap().inserted_tasks.len() > 1);
    }

    #[test]
    fn scheduler_doesnt_send_to_unconfigured_service() {
        let system = System::new("test");

        let messages_received: Arc<Mutex<Vec<ScheduledTaskMessage>>> = Arc::new(Mutex::new(vec![]));

        TestActor {
            messages_recieved: Arc::clone(&messages_received),
        }
        .start();

        Scheduler::test(
            vec![ScheduledTaskConfig {
                cron: "* * * * * * *".to_string(),
                message: ScheduledTaskMessage::FetchNews,
            }],
            Box::new(Arc::new(Mutex::new(TestSchedulerPorts::new()))),
        )
        .start();

        let current = System::current();
        thread::spawn(move || {
            thread::sleep(time::Duration::from_millis(60));
            assert!(messages_received.lock().unwrap().is_empty());
            current.stop();
        });

        system.run().unwrap();
    }
}
