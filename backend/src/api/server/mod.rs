use actix_web::{get, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

/// Retrieves general information about the server.
#[get("server")]
pub async fn handle() -> Res {
    Res {
        version: env!("CARGO_PKG_VERSION").into(),
        features: vec![
            #[cfg(feature = "email_smtp")]
            "email".into(),
        ],
    }
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The server version.
    version: String,

    /// The features enabled for this server.
    features: Vec<String>,
}

impl Responder for Res {
    fn respond_to(self, _request: &HttpRequest) -> HttpResponse {
        HttpResponse::Ok().json(self)
    }
}
