use sqlx::prelude::*;

use std::sync::Arc;

use actix_session::Session;
use actix_web::{post, web, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::db;
use crate::db::entities::{request, Request};
use crate::db::values::{Role, Timestamp, UID};
use crate::notifications::Notifier;

/// Generates a user request.
#[post("request/{user_uid}")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    notifier: web::Data<Arc<Notifier<Event>>>,
    session: Session,
    req: web::Json<Req>,
    user_uid: web::Path<UID>,
) -> impl Responder {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let state = State::load(&session)?;
    {
        let res = execute(
            &mut trans,
            state.clone(),
            &req.into_inner(),
            &user_uid.into_inner(),
        )
        .await?;
        Notify::Parents {
            event: Event::RequestCreated {
                request: res.request.clone(),
                by: state.user_uid.clone(),
            },
            family: state.family_uid,
        }
        .send(&mut *trans, &notifier, &state.user_uid)
        .await;
        trans.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    e: &mut api::Executor<'a>,
    state: State,
    req: &Req,
    user_uid: &UID,
) -> Result<Res, api::Error> {
    state.assert_user(&user_uid)?.assert_role(Role::Child)?;

    let request = Request::create_with_auto_uid(
        &mut *e,
        user_uid.clone(),
        api::argument(req.name.clone())?,
        api::argument(req.description.clone())?,
        api::argument(req.amount)?,
        req.url.clone().flatten(),
        Timestamp::now(),
    )
    .await?;

    Ok(Res { request })
}

pub type Req = request::Description;

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The generated request.
    pub request: Request,
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::entities::Entity;
    use crate::db::test_pool;

    use super::*;

    #[actix_rt::test]
    async fn success() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, _, children, _, _) = tests::populate(&mut c).unwrap();
        let amount = 424242;

        let mut trans = pool.begin().await.unwrap();
        let res = execute(
            &mut trans,
            State {
                user_uid: children.0.uid().clone(),
                role: children.0.role().clone(),
                family_uid: family.uid().clone(),
            },
            &Req {
                name: Some("A name".into()),
                description: Some("A description!".into()),
                amount: Some(amount),
                url: Some(None),
                ..Default::default()
            },
            children.0.uid(),
        )
        .await
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(*res.request.amount(), amount);
        assert_eq!(
            Request::read(&mut c, res.request.uid()).await.unwrap(),
            Some(res.request),
        );
    }

    #[actix_rt::test]
    async fn forbidden() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, _, children, _, _) = tests::populate(&mut c).unwrap();

        let mut trans = pool.begin().await.unwrap();
        let err = execute(
            &mut trans,
            State {
                user_uid: children.1.uid().clone(),
                role: children.1.role().clone(),
                family_uid: family.uid().clone(),
            },
            &Req {
                name: Some("A name".into()),
                description: Some("A description!".into()),
                amount: Some(0),
                url: Some(None),
                ..Default::default()
            },
            children.0.uid(),
        )
        .await
        .err()
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(err, api::Error::forbidden("invalid user"));
    }
}
