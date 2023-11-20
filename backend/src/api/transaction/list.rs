use crate::prelude::*;

use std::ops::Range;

use crate::api;
use crate::api::session::State;
use crate::db;
use crate::db::entities::{Transaction, User};
use crate::db::values::{Role, UID};

/// Retrieves all transactions for a user.
#[get("transaction/{user_uid}")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    session: Session,
    user_uid: web::Path<UID>,
    query: web::Query<Query>,
) -> impl Responder {
    let mut conn = database.connection().await?;
    let mut tx = conn.begin().await?;
    let state = State::load(&session)?;
    {
        let res = execute(
            &mut tx,
            state,
            &query.into_inner(),
            &user_uid.into_inner(),
        )
        .await?;
        tx.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    tx: &mut Tx<'a>,
    state: State,
    query: &Query,
    user_uid: &UID,
) -> Result<Res, api::Error> {
    let user = User::read(tx.as_mut(), &user_uid)
        .await?
        .ok_or_else(|| api::Error::forbidden("invalid user"))?;
    match state.role {
        Role::Parent => state.assert_family(&user.family_uid)?,
        Role::Child => state.assert_user(&user.uid)?,
    };

    let transactions =
        Transaction::read_for_user_limit(tx, &user_uid, query.clone().into())
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

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::entities::create;
    use crate::db::test_engine;
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn success_parent() {
        let database = test_engine().await;
        let mut c = database.connection().await.unwrap();
        let (family, parent, children, transactions, _) =
            tests::populate(&mut c).unwrap();

        let res = {
            let mut tx = c.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    role: parent.role.clone(),
                    family_uid: family.uid.clone(),
                },
                &Query {
                    limit: transactions.len(),
                    offset: 0,
                },
                &children.0.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(res.transactions.len(), transactions.len() / 2);
        for transaction in
            transactions.iter().filter(|r| r.user_uid == children.0.uid)
        {
            assert!(res.transactions.contains(transaction));
        }
    }

    #[actix_rt::test]
    async fn success_child() {
        let database = test_engine().await;
        let mut c = database.connection().await.unwrap();
        let (family, _, children, transactions, _) =
            tests::populate(&mut c).unwrap();

        let res = {
            let mut tx = c.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: children.0.uid.clone(),
                    role: children.0.role.clone(),
                    family_uid: family.uid.clone(),
                },
                &Query {
                    limit: transactions.len(),
                    offset: 0,
                },
                &children.0.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(res.transactions.len(), transactions.len() / 2);
        for transaction in
            transactions.iter().filter(|r| r.user_uid == children.0.uid)
        {
            assert!(res.transactions.contains(transaction));
        }
    }

    #[actix_rt::test]
    async fn unknown_user() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut conn).unwrap();

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    role: parent.role.clone(),
                    family_uid: family.uid.clone(),
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
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(err, api::Error::forbidden("invalid user"));
    }

    #[actix_rt::test]
    async fn forbidden_parent() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut conn).unwrap();
        let other_family = create::family(&mut conn, "Other Family Name");
        let other_user = create::user(
            &mut conn,
            Role::Parent,
            "Other User",
            "other@email.com",
            &other_family.uid,
        );

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    role: parent.role.clone(),
                    family_uid: family.uid.clone(),
                },
                &Query {
                    limit: 100,
                    offset: 0,
                },
                &other_user.uid,
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(err, api::Error::forbidden("invalid family"));
    }

    #[actix_rt::test]
    async fn forbidden_child() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, _, children, _, _) = tests::populate(&mut conn).unwrap();

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: children.0.uid.clone(),
                    role: children.0.role.clone(),
                    family_uid: family.uid.clone(),
                },
                &Query {
                    limit: 100,
                    offset: 0,
                },
                &children.1.uid,
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(err, api::Error::forbidden("invalid user"));
    }
}
