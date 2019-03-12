pub mod messages;
pub mod system;

use self::messages::ScheduleMessage;
use crate::config::{Config, ScheduleConfig};
use crate::error::{Error, Result};
use actix::prelude::*;

#[derive(Eq, PartialEq, Hash)]
pub struct ServiceId(String);
impl ServiceId {
    pub fn from<S: Into<String>>(identifier: S) -> Self {
        ServiceId(identifier.into())
    }
}

pub trait Service {
    fn id() -> ServiceId;
}

pub struct Scheduler {
    config: Vec<ScheduleConfig>,
    services: Vec<Recipient<ScheduleMessage>>,
}
impl Scheduler {
    pub fn new() -> Result<Self> {
        let config = Config::from_file()?.schedule;

        Ok(Scheduler {
            config,
            services: vec![],
        })
    }

    pub fn add_service(&mut self, service: Recipient<ScheduleMessage>) {
        self.services.push(service)
    }
}

impl Service for Scheduler {
    fn id() -> ServiceId {
        ServiceId::from("Scheduler")
    }
}

impl Actor for Scheduler {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        for schedule in &self.config {
            for service in &self.services {
                let schedule = schedule.clone();
                let service = service.clone();
                ctx.run_interval(schedule.interval, move |_, _| {
                    let schedule = schedule.clone();
                    service.do_send(schedule.message).unwrap_or_else(|e| {
                        eprintln!("{}", Into::<Error>::into(e))
                    })
                });
            }
        }
    }
}
