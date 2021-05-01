use sqlx::prelude::*;

use std::sync::Arc;

use actix_session::Session;
use actix_web::{post, web};

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::db;
use crate::notifications::Notifier;

#[post("session/logout")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    notifier: web::Data<Arc<Notifier<Event>>>,
    session: Session,
) -> Result<&'static str, api::Error> {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let state = State::load(&session)?;
    State::clear(&session);

    Notify::Member {
        event: Event::Logout {},
        user: state.user_uid.clone(),
    }
    .send(&mut *trans, &notifier, &state.user_uid)
    .await;

    Ok("logged out")
}
