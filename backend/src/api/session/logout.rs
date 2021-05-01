use actix_session::Session;
use actix_web::post;

use crate::api;

#[post("session/logout")]
pub async fn handle(session: Session) -> Result<&'static str, api::Error> {
    super::State::load(&session)?;
    super::State::clear(&session);

    Ok("logged out")
}
