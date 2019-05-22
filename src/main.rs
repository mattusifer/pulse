#[macro_use]
mod config;
mod constants;
mod db;
mod error;
#[macro_use]
mod services;

use actix::prelude::*;

use crate::{
    error::Result,
    services::{
        broadcast::Broadcast, command::CommandRunner, news::News,
        scheduler::Scheduler, system::SystemMonitor,
    },
};

fn main() -> Result<()> {
    pretty_env_logger::init();

    config::initialize_from_file()?;

    let system = System::new("pulse");

    Broadcast::new().start();

    let monitor_addr = SystemMonitor::new().start();
    let news_addr = News::new().start();
    let command_runner_addr = CommandRunner::new().start();

    let mut scheduler = Scheduler::new();
    scheduler.add_service(Addr::recipient(monitor_addr));
    scheduler.add_service(Addr::recipient(news_addr));
    scheduler.add_service(Addr::recipient(command_runner_addr));
    scheduler.start();

    system.run();

    Ok(())
}
