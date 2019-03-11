mod broadcast;
mod config;
#[macro_use]
mod db;
mod error;
mod services;

use actix::prelude::*;

use crate::error::Result;
use crate::services::system::SystemMonitor;
use crate::services::Scheduler;

fn main() -> Result<()> {
    let system = System::new("pulse");

    let monitor_addr = SystemMonitor::new()?.start();

    let mut scheduler = Scheduler::new()?;
    scheduler.add_service(Addr::recipient(monitor_addr));

    scheduler.start();
    system.run();

    Ok(())
}
