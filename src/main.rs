#[macro_use]
mod config;
mod constants;
mod db;
mod error;
mod services;

use actix::prelude::*;

use crate::{
    error::Result,
    services::{
        broadcast::Broadcast, scheduler::Scheduler, system::SystemMonitor,
    },
};

fn main() -> Result<()> {
    config::initialize_from_file()?;

    let system = System::new("pulse");

    let _broadcast = Broadcast::new().start();

    let monitor_addr = SystemMonitor::new().start();

    let mut scheduler = Scheduler::new();
    scheduler.add_service(Addr::recipient(monitor_addr));
    scheduler.start();

    system.run();

    Ok(())
}
