use crate::prelude::*;

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;

#[post("session/logout")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    channel: web::Data<ChannelEngine>,
    session: Session,
) -> impl Responder {
    let mut conn = database.connection().await?;
    let mut tx = conn.begin().await?;
    let state = State::load(&session)?;
    State::clear(&session);

    Notify::Member {
        event: Event::Logout {},
        user: state.user_uid.clone(),
    }
    .send(&mut tx, &channel, &state.user_uid)
    .await;

    api::ok("logged out")
}
