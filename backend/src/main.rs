use actix::prelude::*;

use std::env;
use std::io;
use std::process::exit;

use actix_web::{App, HttpServer};
use env_logger;

#[macro_use]
mod db;

mod configuration;

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
    HttpServer::new(move || {
        App::new()
            // Grant access to the connection pool
            .data(connection_pool.clone())
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
