use sqlx::prelude::*;

use actix_session::Session;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::session::State;
use crate::db;
use crate::db::entities::{Entity, Request, User};
use crate::db::values::{Role, UID};

/// Retrieves all requests for a user.
#[get("request/{user_uid}/{request_id}")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    session: Session,
    path: web::Path<(UID, i64)>,
) -> Result<Res, api::Error> {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let state = State::load(&session)?;
    let (user_id, request_id) = path.into_inner();
    {
        let res = execute(&mut trans, state, &user_id, &request_id).await?;
        trans.commit().await?;
        Ok(res)
    }
}

pub async fn execute<'a>(
    trans: &mut db::Transaction<'a>,
    state: State,
    user_uid: &UID,
    request_uid: &i64,
) -> Result<Res, api::Error> {
    let user = User::read(&mut *trans, &user_uid)
        .await?
        .ok_or_else(|| api::Error::forbidden("invalid user"))?;
    match state.role {
        Role::Parent => state.assert_family(user.family_uid())?,
        Role::Child => state.assert_user(user.uid())?,
    };

    let request = api::expect(Request::read(&mut *trans, &request_uid).await?)?;
    if request.user_uid() != user.uid() {
        Err(api::Error::not_found("request not found"))
    } else {
        Ok(Res { request })
    }
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The request.
    request: db::entities::Request,
}

impl Responder for Res {
    fn respond_to(self, _request: &HttpRequest) -> HttpResponse {
        HttpResponse::Ok().json(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::entities::create;
    use crate::db::test_pool;
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn success_parent() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, children, _, requests) =
            tests::populate(&mut c).unwrap();
        let request = requests
            .iter()
            .filter(|r| r.user_uid() == children.0.uid())
            .next()
            .unwrap()
            .clone();

        let res = execute(
            &mut pool.begin().await.unwrap(),
            State {
                user_uid: parent.uid().clone(),
                role: parent.role().clone(),
                family_uid: family.uid().clone(),
            },
            children.0.uid(),
            request.uid(),
        )
        .await
        .unwrap();

        assert_eq!(res.request, request);
    }

    #[actix_rt::test]
    async fn success_child() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, _, children, _, requests) =
            tests::populate(&mut c).unwrap();
        let request = requests
            .iter()
            .filter(|r| r.user_uid() == children.0.uid())
            .next()
            .unwrap()
            .clone();

        let res = execute(
            &mut pool.begin().await.unwrap(),
            State {
                user_uid: children.0.uid().clone(),
                role: children.0.role().clone(),
                family_uid: family.uid().clone(),
            },
            children.0.uid(),
            request.uid(),
        )
        .await
        .unwrap();

        assert_eq!(res.request, request);
    }

    #[actix_rt::test]
    async fn unknown_user() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, _, _, requests) = tests::populate(&mut c).unwrap();

        let err = execute(
            &mut pool.begin().await.unwrap(),
            State {
                user_uid: parent.uid().clone(),
                role: parent.role().clone(),
                family_uid: family.uid().clone(),
            },
            &UID::new(),
            requests[0].uid(),
        )
        .await
        .err()
        .unwrap();

        assert_eq!(err, api::Error::forbidden("invalid user"));
    }

    #[actix_rt::test]
    async fn unknown_request() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, children, _, _) = tests::populate(&mut c).unwrap();

        let err = execute(
            &mut pool.begin().await.unwrap(),
            State {
                user_uid: parent.uid().clone(),
                role: parent.role().clone(),
                family_uid: family.uid().clone(),
            },
            children.0.uid(),
            &123456,
        )
        .await
        .err()
        .unwrap();

        assert_eq!(err, api::Error::not_found("not found"));
    }

    #[actix_rt::test]
    async fn forbidden_parent() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut c).unwrap();
        let other_family = create::family(&mut c, "Other Family Name");
        let other_user = create::user(
            &mut c,
            Role::Parent,
            "Other User",
            "other@email.com",
            other_family.uid(),
        );

        let err = execute(
            &mut pool.begin().await.unwrap(),
            State {
                user_uid: parent.uid().clone(),
                role: parent.role().clone(),
                family_uid: family.uid().clone(),
            },
            other_user.uid(),
            &0,
        )
        .await
        .err()
        .unwrap();

        assert_eq!(err, api::Error::forbidden("invalid family"));
    }

    #[actix_rt::test]
    async fn forbidden_child() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, _, children, _, requests) =
            tests::populate(&mut c).unwrap();
        let request = requests
            .iter()
            .filter(|r| r.user_uid() == children.1.uid())
            .next()
            .unwrap()
            .clone();

        let err = execute(
            &mut pool.begin().await.unwrap(),
            State {
                user_uid: children.0.uid().clone(),
                role: children.0.role().clone(),
                family_uid: family.uid().clone(),
            },
            children.1.uid(),
            request.uid(),
        )
        .await
        .err()
        .unwrap();

        assert_eq!(err, api::Error::forbidden("invalid user"));
    }
}
