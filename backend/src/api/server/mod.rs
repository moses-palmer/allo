use actix_web::{get, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

/// Retrieves general information about the server.
#[get("server")]
pub async fn handle() -> Res {
    Res {
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The server version.
    version: String,
}

impl Responder for Res {
    fn respond_to(self, _request: &HttpRequest) -> HttpResponse {
        HttpResponse::Ok().json(self)
    }
}
