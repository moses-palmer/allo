use sqlx::prelude::*;

use actix_session::Session;
use actix_web::{delete, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::session::State;
use crate::db;
use crate::db::entities::{Entity, Request, User};
use crate::db::values::{Role, UID};

/// Deletes a user request
#[delete("request/{user_uid}/{request_uid}")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    session: Session,
    path: web::Path<(UID, i64)>,
) -> Result<Res, api::Error> {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let state = State::load(&session)?;
    let (user_uid, request_uid) = path.into_inner();
    {
        let res = execute(&mut trans, state, &user_uid, &request_uid).await?;
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
    let request = Request::read(&mut *trans, request_uid)
        .await?
        .ok_or_else(|| api::Error::not_found("unknown request"))?;
    let user = User::read(&mut *trans, user_uid)
        .await?
        .ok_or_else(|| api::Error::not_found("unknown request"))?;
    match state.role {
        Role::Parent => state.assert_family(user.family_uid())?,
        Role::Child => state.assert_user(user.uid())?,
    };

    if request.user_uid() != user.uid() {
        Err(api::Error::not_found("unknown request"))
    } else {
        request.delete(&mut *trans).await?;
        Ok(Res { request })
    }
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The request that was declined.
    request: Request,
}

impl Responder for Res {
    fn respond_to(self, _request: &HttpRequest) -> HttpResponse {
        HttpResponse::NoContent().body("deleted")
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
        let candidate = requests
            .iter()
            .filter(|r| r.user_uid() == children.0.uid())
            .next()
            .unwrap()
            .clone();

        let mut trans = pool.begin().await.unwrap();
        let res = execute(
            &mut trans,
            State {
                user_uid: parent.uid().clone(),
                role: parent.role().clone(),
                family_uid: family.uid().clone(),
            },
            children.0.uid(),
            candidate.uid(),
        )
        .await
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(res.request, candidate);
        assert!(Request::read(&mut c, candidate.uid())
            .await
            .unwrap()
            .is_none());
    }

    #[actix_rt::test]
    async fn success_child() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, _, children, _, requests) =
            tests::populate(&mut c).unwrap();
        let candidate = requests
            .iter()
            .filter(|r| r.user_uid() == children.0.uid())
            .next()
            .unwrap()
            .clone();

        let mut trans = pool.begin().await.unwrap();
        let res = execute(
            &mut trans,
            State {
                user_uid: children.0.uid().clone(),
                role: children.0.role().clone(),
                family_uid: family.uid().clone(),
            },
            children.0.uid(),
            candidate.uid(),
        )
        .await
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(res.request, candidate);
        assert!(Request::read(&mut c, candidate.uid())
            .await
            .unwrap()
            .is_none());
    }

    #[actix_rt::test]
    async fn forbidden_parent() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (_, _, children, _, requests) = tests::populate(&mut c).unwrap();
        let other_family = create::family(&mut c, "Other Family");
        let other_parent = create::user(
            &mut c,
            Role::Parent,
            "Other User",
            "other@email.com",
            other_family.uid(),
        );
        let candidate = requests
            .iter()
            .filter(|r| r.user_uid() == children.0.uid())
            .next()
            .unwrap()
            .clone();

        let mut trans = pool.begin().await.unwrap();
        let err = execute(
            &mut trans,
            State {
                user_uid: other_parent.uid().clone(),
                role: other_parent.role().clone(),
                family_uid: other_family.uid().clone(),
            },
            children.0.uid(),
            candidate.uid(),
        )
        .await
        .err()
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(err, api::Error::forbidden("invalid family"));
        assert_eq!(
            Request::read(&mut c, candidate.uid()).await.unwrap(),
            Some(candidate),
        );
    }

    #[actix_rt::test]
    async fn forbidden_child() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, _, children, _, requests) =
            tests::populate(&mut c).unwrap();
        let candidate = requests
            .iter()
            .filter(|r| r.user_uid() == children.0.uid())
            .next()
            .unwrap()
            .clone();

        let mut trans = pool.begin().await.unwrap();
        let err = execute(
            &mut trans,
            State {
                user_uid: children.1.uid().clone(),
                role: children.1.role().clone(),
                family_uid: family.uid().clone(),
            },
            children.0.uid(),
            candidate.uid(),
        )
        .await
        .err()
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(err, api::Error::forbidden("invalid user"));
        assert_eq!(
            Request::read(&mut c, candidate.uid()).await.unwrap(),
            Some(candidate),
        );
    }
}
