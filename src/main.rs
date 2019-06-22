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
mod routes;

// TODO: remove this when diesel is updated for rust 2018:
// https://github.com/diesel-rs/diesel/pull/1956
#[macro_use]
extern crate diesel;

use actix::{Actor, Addr};
use actix_files::Files;
use actix_web::{middleware, App, HttpServer};

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
    log::info!("Database connection initialized");

    HttpServer::new(move || {
        let maybe_broadcast =
            Broadcast::new().expect("Broadcast actor could not be initialized");
        if let Some(broadcast) = maybe_broadcast {
            broadcast.start();
        }
        SystemMonitor::new().start();

        let news_addr = News::new().start();
        let mut scheduler = Scheduler::new();
        scheduler.add_task_runner(Addr::recipient(news_addr));
        scheduler.start();
        log::info!("Scheduler started");

        App::new().wrap(middleware::Logger::default()).service(
            Files::new("/", "./webapp/public/").index_file("index.html"),
        )
    })
    .bind("127.0.0.1:8088")?
    .run()?;

    Ok(())
}
