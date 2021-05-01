use sqlx::prelude::*;

use actix_session::Session;
use actix_web::{post, web, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::session::State;
use crate::db;
use crate::db::entities::{transaction, Entity, Transaction, User};
use crate::db::values::{Role, Timestamp, UID};

/// Generates a transaction.
#[post("transaction/{user_uid}")]
pub async fn handle(
    pool: web::Data<db::Pool>,
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
            state,
            &req.into_inner(),
            &user_uid.into_inner(),
        )
        .await?;
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
    let user = User::read(&mut *e, user_uid)
        .await?
        .ok_or_else(|| api::Error::not_found("unknown request"))?;
    state
        .assert_role(Role::Parent)?
        .assert_family(user.family_uid())?;

    let transaction = Transaction::create_with_auto_uid(
        &mut *e,
        api::argument(req.transaction_type)?,
        user_uid.clone(),
        api::argument(req.description.clone())?,
        api::argument(req.amount)?,
        Timestamp::now(),
    )
    .await?;

    Ok(Res { transaction })
}

pub type Req = transaction::Description;

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The generated transaction.
    pub transaction: Transaction,
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::test_pool;
    use crate::db::values::TransactionType;

    use super::*;

    #[actix_rt::test]
    async fn success() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (_, parent, children, _, _) = tests::populate(&mut c).unwrap();
        let amount = 424242;

        let mut trans = pool.begin().await.unwrap();
        let res = execute(
            &mut trans,
            State {
                user_uid: parent.uid().clone(),
                role: parent.role().clone(),
                family_uid: parent.family_uid().clone(),
            },
            &Req {
                transaction_type: Some(TransactionType::Gift),
                description: Some("A description!".into()),
                amount: Some(amount),
                ..Default::default()
            },
            children.0.uid(),
        )
        .await
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(*res.transaction.transaction_type(), TransactionType::Gift);
        assert_eq!(
            *res.transaction.description(),
            String::from("A description!")
        );
        assert_eq!(*res.transaction.amount(), amount);
        assert_eq!(
            Transaction::read(&mut c, res.transaction.uid())
                .await
                .unwrap(),
            Some(res.transaction),
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
                user_uid: children.0.uid().clone(),
                role: children.0.role().clone(),
                family_uid: family.uid().clone(),
            },
            &Req {
                transaction_type: Some(TransactionType::Gift),
                description: Some("A description!".into()),
                amount: Some(10),
                ..Default::default()
            },
            children.0.uid(),
        )
        .await
        .err()
        .unwrap();
        trans.commit().await.unwrap();

        assert_eq!(err, api::Error::forbidden("invalid role"));
    }
}
