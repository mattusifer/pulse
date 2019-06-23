mod config;
mod constants;
mod db;
mod error;
mod routes;
mod schema;
mod services;

// TODO: remove this when diesel is updated for rust 2018:
// https://github.com/diesel-rs/diesel/pull/1956
#[macro_use]
extern crate diesel;

use std::env;

use actix::{Actor, Addr, System};
use actix_files::Files;
use actix_web::{middleware, web, App, HttpServer};
use actix_web_actors::ws;

use crate::{
    error::Result,
    routes::Ws,
    services::{
        broadcast::Broadcast, news::News, scheduler::Scheduler,
        system::SystemMonitor,
    },
};

fn main() -> Result<()> {
    env::set_var("RUST_LOG", "actix_server=info,actix_web=info,pulse=info");
    pretty_env_logger::init();

    config::initialize_from_file()?;
    db::initialize_postgres()?;
    log::info!("Database connection initialized");

    let system = System::new("pulse");

    // Only start broadcast if alerts have been configured
    // (it will be defined otherwise)
    Broadcast::new()?.map(|b| b.start());
    let monitor = SystemMonitor::new().start();

    let news_addr = News::new().start();
    let mut scheduler = Scheduler::new();
    scheduler.add_task_runner(Addr::recipient(news_addr));
    scheduler.start();
    log::info!("Scheduler started");

    HttpServer::new(move || {
        let monitor = monitor.clone();

        App::new()
            .wrap(middleware::Logger::default())
            // websocket
            .service(web::resource("/ws").route(web::get().to(
                move |request, stream: web::Payload| {
                    ws::start(Ws::new(monitor.clone()), &request, stream)
                },
            )))
        // // index
        // .service(
        //     Files::new("/", "./webapp/public/").index_file("index.html"),
        // )
    })
    .bind("127.0.0.1:8088")?
    .start();

    system.run()?;

    Ok(())
}
