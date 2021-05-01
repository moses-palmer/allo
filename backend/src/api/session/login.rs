use actix_session::Session;
use actix_web::{post, web, Responder};
use serde::{Deserialize, Serialize};
use sqlx::prelude::*;

use crate::api;
use crate::db;
use crate::db::entities::{Entity, Password, User};
use crate::db::values::{EmailAddress, UID};

#[post("session/login")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    session: Session,
    req: web::Json<Req>,
) -> impl Responder {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    {
        let res = execute(&mut trans, &req.into_inner()).await?;
        trans.commit().await?;
        super::State {
            user_uid: res.user.uid().clone(),
            family_uid: res.user.family_uid().clone(),
            role: res.user.role().clone(),
        }
        .store(&session)?;

        api::ok(res)
    }
}

/// Logs in a user.
///
/// # Arguments
/// *  `e` - The database executor.
/// *  `user_uid` - The user unique identifier.
/// *  `password` - The password to use.
pub async fn execute<'a>(
    e: &mut api::Executor<'a>,
    req: &Req,
) -> Result<Res, api::Error> {
    use UserIdentifier::*;
    let password_hash = match req.identifier {
        Email { ref email } => Password::read_by_email(&mut *e, email).await?,
        UID { ref uid } => Password::read(&mut *e, uid).await?,
    }
    .ok_or_else(api::Error::unauthorized)?;
    if password_hash.hash().verify(&req.password).unwrap_or(false) {
        let user = User::read(&mut *e, password_hash.user_uid())
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
    use crate::db::test_pool;

    use super::*;

    #[actix_rt::test]
    async fn success_email() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (_, user, _, _, _) = tests::populate(&mut c).unwrap();
        create::password(&mut c, "password", user.uid());

        let res = execute(
            &mut pool.begin().await.unwrap(),
            &Req {
                identifier: UserIdentifier::Email {
                    email: user.email().clone().unwrap(),
                },
                password: "password".into(),
            },
        )
        .await
        .unwrap();

        assert_eq!(res.user, user);
    }

    #[actix_rt::test]
    async fn success_uid() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (_, user, _, _, _) = tests::populate(&mut c).unwrap();
        create::password(&mut c, "password", user.uid());

        let res = execute(
            &mut pool.begin().await.unwrap(),
            &Req {
                identifier: UserIdentifier::UID {
                    uid: user.uid().clone(),
                },
                password: "password".into(),
            },
        )
        .await
        .unwrap();

        assert_eq!(res.user, user);
    }

    #[actix_rt::test]
    async fn invalid_credentials_email() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (_, user, _, _, _) = tests::populate(&mut c).unwrap();
        create::password(&mut c, "password", user.uid());

        let err = execute(
            &mut pool.begin().await.unwrap(),
            &Req {
                identifier: UserIdentifier::Email {
                    email: user.email().clone().unwrap(),
                },
                password: "invalid".into(),
            },
        )
        .await
        .err()
        .unwrap();

        assert_eq!(
            err,
            api::Error::Static(StatusCode::UNAUTHORIZED, "unauthorized"),
        );
    }

    #[actix_rt::test]
    async fn invalid_credentials_uid() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (_, user, _, _, _) = tests::populate(&mut c).unwrap();
        create::password(&mut c, "password", user.uid());

        let err = execute(
            &mut pool.begin().await.unwrap(),
            &Req {
                identifier: UserIdentifier::UID {
                    uid: user.uid().clone(),
                },
                password: "invalid".into(),
            },
        )
        .await
        .err()
        .unwrap();

        assert_eq!(
            err,
            api::Error::Static(StatusCode::UNAUTHORIZED, "unauthorized"),
        );
    }
}
