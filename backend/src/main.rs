use actix::prelude::*;

use std::io;
use std::process::exit;

use actix_web::web::Data;
use actix_web::{App, HttpServer};
use env_logger;

async fn run() -> io::Result<()> {
    env_logger::builder().format_timestamp(None).init();

    HttpServer::new(|| App::new())
        .bind("0.0.0.0:8000")
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
