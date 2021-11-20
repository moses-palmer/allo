use std::ops::Range;

use sqlx::prelude::*;

use actix_session::Session;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::session::State;
use crate::db;
use crate::db::entities::{Entity, Transaction, User};
use crate::db::values::{Role, UID};

/// Retrieves all transactions for a user.
#[get("transaction/{user_uid}")]
pub async fn handle(
    pool: web::Data<db::Pool>,
    session: Session,
    user_uid: web::Path<UID>,
    query: web::Query<Query>,
) -> Result<Res, api::Error> {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let state = State::load(&session)?;
    {
        let res = execute(
            &mut trans,
            state,
            &query.into_inner(),
            &user_uid.into_inner(),
        )
        .await?;
        trans.commit().await?;
        Ok(res)
    }
}

pub async fn execute<'a>(
    e: &mut api::Executor<'a>,
    state: State,
    query: &Query,
    user_uid: &UID,
) -> Result<Res, api::Error> {
    let user = User::read(&mut *e, &user_uid)
        .await?
        .ok_or_else(|| api::Error::forbidden("invalid user"))?;
    match state.role {
        Role::Parent => state.assert_family(user.family_uid())?,
        Role::Child => state.assert_user(user.uid())?,
    };

    let transactions = Transaction::read_for_user_limit(
        &mut *e,
        &user_uid,
        query.clone().into(),
    )
    .await?;

    Ok(Res { transactions })
}

#[derive(Clone, Deserialize)]
pub struct Query {
    /// The maximum number of transactions to return.
    limit: usize,

    /// The offset from which to strat.
    offset: usize,
}

impl From<Query> for Range<usize> {
    fn from(source: Query) -> Self {
        source.offset..(source.offset + source.limit)
    }
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// All outstanding requests.
    transactions: Vec<db::entities::Transaction>,
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
        let (family, parent, children, transactions, _) =
            tests::populate(&mut c).unwrap();

        let res = execute(
            &mut pool.begin().await.unwrap(),
            State {
                user_uid: parent.uid().clone(),
                role: parent.role().clone(),
                family_uid: family.uid().clone(),
            },
            &Query {
                limit: transactions.len(),
                offset: 0,
            },
            children.0.uid(),
        )
        .await
        .unwrap();

        assert_eq!(res.transactions.len(), transactions.len() / 2);
        for transaction in transactions
            .iter()
            .filter(|r| r.user_uid() == children.0.uid())
        {
            assert!(res.transactions.contains(transaction));
        }
    }

    #[actix_rt::test]
    async fn success_child() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, _, children, transactions, _) =
            tests::populate(&mut c).unwrap();

        let res = execute(
            &mut pool.begin().await.unwrap(),
            State {
                user_uid: children.0.uid().clone(),
                role: children.0.role().clone(),
                family_uid: family.uid().clone(),
            },
            &Query {
                limit: transactions.len(),
                offset: 0,
            },
            children.0.uid(),
        )
        .await
        .unwrap();

        assert_eq!(res.transactions.len(), transactions.len() / 2);
        for transaction in transactions
            .iter()
            .filter(|r| r.user_uid() == children.0.uid())
        {
            assert!(res.transactions.contains(transaction));
        }
    }

    #[actix_rt::test]
    async fn unknown_user() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut c).unwrap();

        let err = execute(
            &mut pool.begin().await.unwrap(),
            State {
                user_uid: parent.uid().clone(),
                role: parent.role().clone(),
                family_uid: family.uid().clone(),
            },
            &Query {
                limit: 100,
                offset: 0,
            },
            &UID::new(),
        )
        .await
        .err()
        .unwrap();

        assert_eq!(err, api::Error::forbidden("invalid user"));
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
            &Query {
                limit: 100,
                offset: 0,
            },
            other_user.uid(),
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
        let (family, _, children, _, _) = tests::populate(&mut c).unwrap();

        let err = execute(
            &mut pool.begin().await.unwrap(),
            State {
                user_uid: children.0.uid().clone(),
                role: children.0.role().clone(),
                family_uid: family.uid().clone(),
            },
            &Query {
                limit: 100,
                offset: 0,
            },
            children.1.uid(),
        )
        .await
        .err()
        .unwrap();

        assert_eq!(err, api::Error::forbidden("invalid user"));
    }
}
