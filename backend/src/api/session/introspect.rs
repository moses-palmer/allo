use sqlx::prelude::*;

use actix_session::Session;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::session::State;
use crate::db;
use crate::db::entities::{Entity, User};
use crate::db::values::UID;

/// Introspects the current session.
#[get("session/introspect")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    session: Session,
) -> Result<Res, api::Error> {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let state = State::load(&session)?;
    {
        let user =
            execute(&mut trans, &state.user_uid).await?.ok_or_else(|| {
                api::Error::Static(StatusCode::UNAUTHORIZED, "unknown user")
            })?;

        Ok(Res { user })
    }
}

pub async fn execute<'a>(
    trans: &mut db::Transaction<'a>,
    user_uid: &UID,
) -> Result<Option<User>, api::Error> {
    Ok(User::read(&mut *trans, user_uid).await?)
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The currently logged in user.
    user: User,
}

impl Responder for Res {
    fn respond_to(self, _request: &HttpRequest) -> HttpResponse {
        HttpResponse::Ok().json(self)
    }
}
