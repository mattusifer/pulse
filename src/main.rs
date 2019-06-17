#[macro_use]
mod config;
mod constants;
#[macro_use]
mod db;
mod error;
#[macro_use]
mod schema;
#[macro_use]
mod services;

// TODO: remove this when diesel is updated for rust 2018:
// https://github.com/diesel-rs/diesel/pull/1956
#[macro_use]
extern crate diesel;

use actix::prelude::*;

use crate::{
    error::Result,
    services::{
        broadcast::Broadcast, news::News, scheduler::Scheduler,
        system::SystemMonitor,
    },
};

fn main() -> Result<()> {
    pretty_env_logger::init();

    config::initialize_from_file()?;
    db::initialize_postgres()?;

    let system = System::new("pulse");

    Broadcast::new().start();

    let monitor_addr = SystemMonitor::new().start();
    let news_addr = News::new().start();

    let mut scheduler = Scheduler::new();
    scheduler.add_service(Addr::recipient(monitor_addr));
    scheduler.add_service(Addr::recipient(news_addr));
    scheduler.start();

    system.run()?;

    Ok(())
}
