use crate::prelude::*;

use crate::api;

/// Retrieves general information about the server.
#[get("server")]
pub async fn handle() -> impl Responder {
    api::ok(Res {
        version: env!("CARGO_PKG_VERSION").into(),
        features: vec![
            #[cfg(feature = "email-smtp")]
            "email".into(),
        ],
    })
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The server version.
    version: String,

    /// The features enabled for this server.
    features: Vec<String>,
}
