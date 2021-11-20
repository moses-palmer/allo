use sqlx::prelude::*;

use std::sync::Arc;

use actix_session::Session;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::notify::{Event, Notify};
use crate::api::session::State;
use crate::db;
use crate::db::entities::{Entity, Request, Transaction, User};
use crate::db::values::{Role, Timestamp, TransactionType, UID};
use crate::notifications::Notifier;

/// Grants a request.
#[post("request/{user_uid}/{request_uid}")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    notifier: web::Data<Arc<Notifier<Event>>>,
    session: Session,
    req: web::Json<Req>,
    path: web::Path<(UID, i64)>,
) -> Result<Res, api::Error> {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let state = State::load(&session)?;
    let (user_uid, request_uid) = path.into_inner();
    {
        let res = execute(
            &mut trans,
            state.clone(),
            &req.into_inner(),
            &user_uid,
            &request_uid,
        )
        .await?;
        Notify::MemberAndParents {
            event: Event::RequestGranted {
                request: res.request.clone(),
                by: state.user_uid.clone(),
            },
            uid: user_uid,
            family: state.family_uid,
        }
        .send(&mut *trans, &notifier, &state.user_uid)
        .await;
        trans.commit().await?;
        Ok(res)
    }
}

pub async fn execute<'a>(
    e: &mut api::Executor<'a>,
    state: State,
    req: &Req,
    user_uid: &UID,
    request_uid: &i64,
) -> Result<Res, api::Error> {
    let request = Request::read(&mut *e, request_uid)
        .await?
        .ok_or_else(|| api::Error::not_found("unknown request"))?;
    let user = User::read(&mut *e, user_uid)
        .await?
        .ok_or_else(|| api::Error::not_found("unknown request"))?;
    state
        .assert_role(Role::Parent)?
        .assert_family(user.family_uid())?;

    if request.user_uid() != user.uid() {
        Err(api::Error::not_found("unknown request"))
    } else {
        let transaction = Transaction::create_with_auto_uid(
            &mut *e,
            TransactionType::Request,
            user.uid().clone(),
            request.name().clone(),
            -(req.cost.unwrap_or(*request.amount()) as i64),
            Timestamp::now(),
        )
        .await?;
        request.delete(&mut *e).await?;
        Ok(Res {
            request,
            transaction,
        })
    }
}

#[derive(Deserialize, Serialize)]
pub struct Req {
    /// The actual, reviewed cost of this item.
    ///
    /// If this is not present, the value from the database is used.
    pub cost: Option<i64>,
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The request that was granted.
    pub request: Request,

    /// The generated transaction.
    pub transaction: Transaction,
}

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
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn success_no_cost() {
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
            &Req { cost: None },
            children.0.uid(),
            candidate.uid(),
        )
        .await
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(*res.transaction.amount(), -candidate.amount());
        assert!(Request::read(&mut c, candidate.uid())
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            Transaction::read(&mut c, res.transaction.uid())
                .await
                .unwrap(),
            Some(res.transaction),
        );
    }

    #[actix_rt::test]
    async fn success_with_cost() {
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
        let cost = 12345678;

        let mut trans = pool.begin().await.unwrap();
        let res = execute(
            &mut trans,
            State {
                user_uid: parent.uid().clone(),
                role: parent.role().clone(),
                family_uid: family.uid().clone(),
            },
            &Req { cost: Some(cost) },
            children.0.uid(),
            candidate.uid(),
        )
        .await
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(*res.transaction.amount(), -cost);
        assert!(Request::read(&mut c, candidate.uid())
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            Transaction::read(&mut c, res.transaction.uid())
                .await
                .unwrap(),
            Some(res.transaction),
        );
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
            &Req { cost: None },
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
            &Req { cost: None },
            children.0.uid(),
            candidate.uid(),
        )
        .await
        .err()
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(err, api::Error::forbidden("invalid role"));
        assert_eq!(
            Request::read(&mut c, candidate.uid()).await.unwrap(),
            Some(candidate),
        );
    }
}
