use weru::actix::prelude::*;

use std::env;
use std::error::Error;
use std::process::exit;

use weru::actix::web::web::Data;
use weru::actix::web::{App, HttpServer};
use weru::env_logger;

#[macro_use]
mod db;

mod api;
mod configuration;
mod tasks;

mod prelude;

async fn run() -> Result<(), Box<dyn Error>> {
    env_logger::builder().format_timestamp(None).init();

    let configuration = configuration::Configuration::load(
        &env::var("ALLO_CONFIGURATION_FILE")
            .map_err(|_| "ALLO_CONFIGURATION_FILE not set".to_string())?,
    )
    .map_err(|e| format!("failed to load configuration: {}", e))?;

    let bind = configuration.server_bind();

    let session_store = configuration.session.store().await?;

    let database = Data::new(configuration.database.engine().await?);
    db::MIGRATOR.run(&mut database.connection().await?).await?;

    let tasks_connection_pool = configuration.database.engine().await?;
    let _scheduler = Supervisor::start(|_| {
        tasks::Scheduled::new(tasks_connection_pool).with(
            tasks::ScheduledTask::Daily(Box::new(tasks::allowance::Payer)),
        )
    });

    let channel = Data::new(configuration.channel.engine().await?);

    let email = configuration.email.engine().await?;
    let sender = Data::new(email.sender().await);

    let defaults = Data::new(configuration.defaults());
    let configuration = Data::new(configuration);

    Ok(HttpServer::new(move || {
        App::new()
            .app_data(configuration.clone())
            .app_data(database.clone())
            .app_data(defaults.clone())
            .app_data(channel.clone())
            .app_data(sender.clone())
            .wrap(session_store.clone().middleware(&configuration.session))
            .service(api::server::handle)
            .service(api::family::add::handle)
            .service(api::family::register::handle)
            .service(api::family::remove::handle)
            .service(api::invitation::accept::handle)
            .service(api::invitation::create::handle)
            .service(api::invitation::get::handle)
            .service(api::overview::handle)
            .service(api::request::decline::handle)
            .service(api::request::get::handle)
            .service(api::request::grant::handle)
            .service(api::request::make::handle)
            .service(api::session::introspect::handle)
            .service(api::session::login::handle)
            .service(api::session::logout::handle)
            .service(api::session::password::handle)
            .service(api::transaction::create::handle)
            .service(api::transaction::list::handle)
            .service(api::user::allowance::handle)
            .service(api::user::get::handle)
            .route(
                "/notify",
                weru::actix::web::web::get().to(api::notify::handle),
            )
    })
    .bind(bind)
    .unwrap()
    .run()
    .await?)
}

#[weru::main]
async fn main() {
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
