use crate::prelude::*;

use crate::api;
use crate::db::entities::{Password, User};
use crate::db::values::{EmailAddress, UID};

#[post("session/login")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    session: Session,
    req: web::Json<Req>,
) -> impl Responder {
    let mut conn = database.connection().await?;
    let mut tx = conn.begin().await?;
    {
        let res = execute(&mut tx, &req.into_inner()).await?;
        tx.commit().await?;
        super::State {
            user_uid: res.user.uid.clone(),
            family_uid: res.user.family_uid.clone(),
            role: res.user.role.clone(),
        }
        .store(&session)?;

        api::ok(res)
    }
}

/// Logs in a user.
///
/// # Arguments
/// *  `tx` - The database transaction.
/// *  `user_uid` - The user unique identifier.
/// *  `password` - The password to use.
pub async fn execute<'a>(
    tx: &mut Tx<'a>,
    req: &Req,
) -> Result<Res, api::Error> {
    use UserIdentifier::*;
    let password_hash = match req.identifier {
        Email { ref email } => Password::read_by_email(tx, email).await?,
        UID { ref uid } => Password::read(tx.as_mut(), uid).await?,
    }
    .ok_or_else(api::Error::unauthorized)?;
    if password_hash.hash.verify(&req.password).unwrap_or(false) {
        let user = User::read(tx.as_mut(), &password_hash.user_uid)
            .await?
            .ok_or_else(api::Error::unauthorized)?;
        Ok(Res { user })
    } else {
        Err(api::Error::unauthorized())
    }
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum UserIdentifier {
    /// The user email address.
    Email { email: EmailAddress },

    /// The user unique identifier.
    UID { uid: UID },
}

#[derive(Deserialize, Serialize)]
pub struct Req {
    /// The user identifier.
    #[serde(flatten)]
    pub identifier: UserIdentifier,

    /// The password.
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The logged in user.
    pub user: User,
}

#[cfg(test)]
mod tests {
    use actix_web::http::StatusCode;

    use crate::api::tests;
    use crate::db::entities::create;
    use crate::db::test_engine;

    use super::*;

    #[actix_rt::test]
    async fn success_email() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (_, user, _, _, _) = tests::populate(&mut conn).unwrap();
        create::password(&mut conn, "password", &user.uid);

        let res = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                &Req {
                    identifier: UserIdentifier::Email {
                        email: user.email.clone().unwrap(),
                    },
                    password: "password".into(),
                },
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(res.user, user);
    }

    #[actix_rt::test]
    async fn success_uid() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (_, user, _, _, _) = tests::populate(&mut conn).unwrap();
        create::password(&mut conn, "password", &user.uid);

        let res = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                &Req {
                    identifier: UserIdentifier::UID {
                        uid: user.uid.clone(),
                    },
                    password: "password".into(),
                },
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(res.user, user);
    }

    #[actix_rt::test]
    async fn invalid_credentials_email() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (_, user, _, _, _) = tests::populate(&mut conn).unwrap();
        create::password(&mut conn, "password", &user.uid);
        let mut tx = conn.begin().await.unwrap();

        let err = {
            let r = execute(
                &mut tx,
                &Req {
                    identifier: UserIdentifier::Email {
                        email: user.email.clone().unwrap(),
                    },
                    password: "invalid".into(),
                },
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(
            err,
            api::Error::Static(StatusCode::UNAUTHORIZED, "unauthorized"),
        );
    }

    #[actix_rt::test]
    async fn invalid_credentials_uid() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (_, user, _, _, _) = tests::populate(&mut conn).unwrap();
        create::password(&mut conn, "password", &user.uid);

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                &Req {
                    identifier: UserIdentifier::UID {
                        uid: user.uid.clone(),
                    },
                    password: "invalid".into(),
                },
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(
            err,
            api::Error::Static(StatusCode::UNAUTHORIZED, "unauthorized"),
        );
    }
}
