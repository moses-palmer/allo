use actix_web::{get, Responder};
use serde::{Deserialize, Serialize};

use crate::api;

/// Retrieves general information about the server.
#[get("server")]
pub async fn handle() -> impl Responder {
    api::ok(Res {
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The server version.
    version: String,
}
