use sqlx::prelude::*;

use std::sync::Arc;

use actix_session::Session;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::db;
use crate::db::entities::{Entity, Password};
use crate::db::values::PasswordHash;
use crate::notifications::Notifier;

/// Changes the password for a user.
#[post("session/password")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    notifier: web::Data<Arc<Notifier<Event>>>,
    session: Session,
    req: web::Json<Req>,
) -> Result<Res, api::Error> {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let state = State::load(&session)?;
    {
        let res = execute(&mut trans, state.clone(), &req.into_inner()).await?;
        Notify::Member {
            event: Event::Logout {},
            user: state.user_uid.clone(),
        }
        .send(&mut *trans, &notifier, &state.user_uid)
        .await;
        trans.commit().await?;
        super::State::clear(&session);
        Ok(res)
    }
}

pub async fn execute<'a>(
    e: &mut api::Executor<'a>,
    state: State,
    req: &Req,
) -> Result<Res, api::Error> {
    let password =
        api::argument(Password::read(&mut *e, &state.user_uid).await?)?;
    if password
        .hash()
        .verify(&req.current_password)
        .unwrap_or(false)
    {
        Password::new(
            password.user_uid().clone(),
            api::argument(PasswordHash::from_password(&req.new_password))?,
        )
        .update(&mut *e)
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

impl Responder for Res {
    fn respond_to(self, _request: &HttpRequest) -> HttpResponse {
        HttpResponse::Created().json(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::entities::create;
    use crate::db::test_pool;

    use super::*;

    #[actix_rt::test]
    async fn success() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut c).unwrap();
        create::password(&mut c, "123", parent.uid());

        let mut trans = c.begin().await.unwrap();
        execute(
            &mut trans,
            State {
                user_uid: parent.uid().clone(),
                family_uid: family.uid().clone(),
                role: parent.role().clone(),
            },
            &Req {
                current_password: "123".into(),
                new_password: "456".into(),
            },
        )
        .await
        .unwrap();
        trans.commit().await.unwrap();

        let password =
            Password::read(&mut c, parent.uid()).await.unwrap().unwrap();
        assert!(password.hash().verify("456").unwrap());
    }

    #[actix_rt::test]
    async fn forbidden() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut c).unwrap();
        create::password(&mut c, "123", parent.uid());

        let mut trans = c.begin().await.unwrap();
        let err = execute(
            &mut trans,
            State {
                user_uid: parent.uid().clone(),
                family_uid: family.uid().clone(),
                role: parent.role().clone(),
            },
            &Req {
                current_password: "456".into(),
                new_password: "789".into(),
            },
        )
        .await
        .err()
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(err, api::Error::forbidden("invalid password"));
    }
}
