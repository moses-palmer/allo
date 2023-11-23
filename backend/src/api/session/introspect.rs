use crate::prelude::*;

use crate::api;
use crate::api::session::State;
use crate::db::entities::User;
use crate::db::values::UID;

/// Introspects the current session.
#[get("session/introspect")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    session: Session,
) -> impl Responder {
    let mut conn = database.connection().await?;
    let mut tx = conn.begin().await?;
    let state = State::load(&session)?;
    {
        let user =
            execute(&mut tx, &state.user_uid).await?.ok_or_else(|| {
                api::Error::Static(StatusCode::UNAUTHORIZED, "unknown user")
            })?;

        api::ok(Res { user })
    }
}

pub async fn execute<'a>(
    tx: &mut Tx<'a>,
    user_uid: &UID,
) -> Result<Option<User>, api::Error> {
    Ok(User::read(tx.as_mut(), user_uid).await?)
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The currently logged in user.
    user: User,
}
