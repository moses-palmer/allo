use actix::prelude::*;

use std::env;
use std::io;
use std::process::exit;

use actix_web::web::Data;
use actix_web::{App, HttpServer};
use env_logger;

#[macro_use]
mod db;

mod api;
mod configuration;
mod tasks;

async fn run() -> io::Result<()> {
    env_logger::builder().format_timestamp(None).init();

    let configuration = configuration::Configuration::load(
        &env::var("ALLO_CONFIGURATION_FILE")
            .expect("ALLO_CONFIGURATION_FILE not set"),
    )?;
    let bind = configuration.server_bind();
    let connection_pool = configuration
        .connection_pool()
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    db::MIGRATOR
        .run(&connection_pool)
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let tasks_connection_pool = connection_pool.clone();
    let _scheduler = Supervisor::start(|_| {
        tasks::Scheduled::new(tasks_connection_pool).with(
            tasks::ScheduledTask::Daily(Box::new(tasks::allowance::Payer)),
        )
    });
    HttpServer::new(move || {
        App::new()
            // Grant access to the connection pool
            .app_data(Data::new(connection_pool.clone()))
            // Persist session
            .wrap(configuration.session())
    })
    .bind(bind)
    .unwrap()
    .run()
    .await
}

#[actix_web::main]
async fn main() {
    use std::error::Error;
    match run().await {
        Err(e) => {
            eprintln!("Failed to run: {}", e);
            let mut error = e.source();
            while let Some(e) = error {
                eprintln!("Caused by: {}", e);
                error = e.source();
            }
            exit(1);
        }
        Ok(_) => {}
    }
}
