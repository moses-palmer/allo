use actix_session::Session;
use actix_web::{post, Responder};

use crate::api;

#[post("session/logout")]
pub async fn handle(session: Session) -> impl Responder {
    super::State::load(&session)?;
    super::State::clear(&session);

    api::ok("logged out")
}
