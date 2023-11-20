use crate::prelude::*;

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::db::entities::Password;
use crate::db::values::PasswordHash;

/// Changes the password for a user.
#[post("session/password")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    channel: web::Data<ChannelEngine>,
    session: Session,
    req: web::Json<Req>,
) -> impl Responder {
    let mut conn = database.connection().await?;
    let mut tx = conn.begin().await?;
    let state = State::load(&session)?;
    {
        let res = execute(&mut tx, state.clone(), &req.into_inner()).await?;
        Notify::Member {
            event: Event::Logout {},
            user: state.user_uid.clone(),
        }
        .send(&mut tx, &channel, &state.user_uid)
        .await;
        tx.commit().await?;
        super::State::clear(&session);
        api::ok(res)
    }
}

pub async fn execute<'a>(
    tx: &mut Tx<'a>,
    state: State,
    req: &Req,
) -> Result<Res, api::Error> {
    let password =
        api::argument(Password::read(tx.as_mut(), &state.user_uid).await?)?;
    if password.hash.verify(&req.current_password).unwrap_or(false) {
        Password::new(
            password.user_uid.clone(),
            api::argument(PasswordHash::from_password(&req.new_password))?,
        )
        .update(tx.as_mut())
        .await?;
        Ok(Res)
    } else {
        Err(api::Error::forbidden("invalid password"))
    }
}

#[derive(Deserialize, Serialize)]
pub struct Req {
    /// The current user password.
    pub current_password: String,

    /// The new user password.
    pub new_password: String,
}

#[derive(Deserialize, Serialize)]
pub struct Res;

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::entities::create;
    use crate::db::test_engine;

    use super::*;

    #[actix_rt::test]
    async fn success() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut conn).unwrap();
        create::password(&mut conn, "123", &parent.uid);

        {
            let mut tx = conn.begin().await.unwrap();
            execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    family_uid: family.uid.clone(),
                    role: parent.role.clone(),
                },
                &Req {
                    current_password: "123".into(),
                    new_password: "456".into(),
                },
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
        }

        let password = Password::read(conn.as_mut(), &parent.uid)
            .await
            .unwrap()
            .unwrap();
        assert!(password.hash.verify("456").unwrap());
    }

    #[actix_rt::test]
    async fn forbidden() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut conn).unwrap();
        create::password(&mut conn, "123", &parent.uid);

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    family_uid: family.uid.clone(),
                    role: parent.role.clone(),
                },
                &Req {
                    current_password: "456".into(),
                    new_password: "789".into(),
                },
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(err, api::Error::forbidden("invalid password"));
    }
}
