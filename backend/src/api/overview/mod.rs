use sqlx::prelude::*;

use std::collections::HashMap;

use actix_session::Session;
use actix_web::{get, web, Responder};
use serde::{Deserialize, Serialize};

use crate::api;
use crate::api::session::State;
use crate::configuration::FamilyConfiguration;
use crate::db;
use crate::db::entities::{Entity, Family, Request, Transaction, User};
use crate::db::values::{Role, UID};

/// The maximum number of transactions to return per user.
const TRANSACTION_LIMIT: usize = 5;

/// Retrieves an overview of the account.
#[get("overview/{family_uid}")]
pub async fn handle(
    defaults: web::Data<FamilyConfiguration>,
    pool: web::Data<db::Pool>,
    session: Session,
    family_uid: web::Path<UID>,
) -> impl Responder {
    let mut connection = pool.acquire().await?;
    let mut trans = connection.begin().await?;
    let state = State::load(&session)?;
    {
        let res = execute(
            &mut trans,
            (*defaults.into_inner()).clone(),
            state,
            family_uid.into_inner(),
        )
        .await?;
        trans.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    e: &mut api::Executor<'a>,
    defaults: FamilyConfiguration,
    state: State,
    family_uid: UID,
) -> Result<Res, api::Error> {
    let State { user_uid, role, .. } = state.assert_family(&family_uid)?;
    let configuration = FamilyConfiguration::read(&mut *e, &family_uid)
        .await?
        .unwrap_or(defaults);
    let family = api::expect(Family::read(&mut *e, &family_uid).await?)?;
    let members = User::read_for_family(&mut *e, &family_uid).await?;
    let requests = match role {
        Role::Parent => Request::read_for_family(&mut *e, &family_uid).await?,
        Role::Child => Request::read_for_user(&mut *e, &user_uid).await?,
    };
    let children = || {
        members.iter().filter(|user| match (role, user.role()) {
            (Role::Parent, Role::Child) => true,
            (Role::Child, _) => user.uid() == &user_uid,
            _ => false,
        })
    };
    let transactions = {
        let mut transactions = Vec::new();
        for child in children() {
            transactions.extend(
                Transaction::read_for_user_limit(
                    &mut *e,
                    child.uid(),
                    0..TRANSACTION_LIMIT,
                )
                .await?,
            );
        }
        transactions
    };
    let balances = {
        let mut balances = HashMap::new();
        for child in children() {
            balances.insert(
                child.uid().clone(),
                Transaction::balance(&mut *e, child.uid())
                    .await?
                    .unwrap_or(0),
            );
        }
        balances
    };

    Ok(Res {
        currency: configuration.currency().clone(),
        family,
        members,
        requests,
        transactions,
        balances,
    })
}

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The currency of all monetary values.
    currency: db::entities::Currency,

    /// The family.
    family: Family,

    /// All members of this family.
    members: Vec<User>,

    /// All outstanding requests.
    requests: Vec<db::entities::Request>,

    /// The most recent transactions for the children.
    transactions: Vec<db::entities::Transaction>,

    /// The balances of the child accounts.
    balances: HashMap<UID, i64>,
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::entities::create;
    use crate::db::test_pool;
    use crate::db::values::CurrencyFormat;

    use super::*;

    #[actix_rt::test]
    async fn success_parent() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, children, transactions, requests) =
            tests::populate(&mut c).unwrap();

        let res = execute(
            &mut pool.begin().await.unwrap(),
            FamilyConfiguration::new(db::entities::Currency::new(
                "TST".into(),
                CurrencyFormat::new("#{}"),
            )),
            State {
                user_uid: parent.uid().clone(),
                role: parent.role().clone(),
                family_uid: family.uid().clone(),
            },
            family.uid().clone(),
        )
        .await
        .unwrap();

        assert_eq!(res.family, family);
        assert_eq!(res.members.len(), 3);
        assert!(res.members.contains(&parent));
        assert!(res.members.contains(&children.0));
        assert!(res.members.contains(&children.1));
        assert_eq!(res.requests.len(), requests.len());
        for request in &requests {
            assert!(res.requests.contains(request));
        }
        assert_eq!(res.transactions.len(), 2 * TRANSACTION_LIMIT);
        for transaction in
            &transactions[transactions.len() - 2 * TRANSACTION_LIMIT..]
        {
            assert!(res.transactions.contains(transaction));
        }
    }

    #[actix_rt::test]
    async fn success_child() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, children, transactions, requests) =
            tests::populate(&mut c).unwrap();

        let res = execute(
            &mut pool.begin().await.unwrap(),
            FamilyConfiguration::new(db::entities::Currency::new(
                "TST".into(),
                CurrencyFormat::new("#{}"),
            )),
            State {
                user_uid: children.0.uid().clone(),
                role: children.0.role().clone(),
                family_uid: family.uid().clone(),
            },
            family.uid().clone(),
        )
        .await
        .unwrap();

        assert_eq!(res.family, family);
        assert_eq!(res.members.len(), 3);
        assert!(res.members.contains(&parent));
        assert!(res.members.contains(&children.0));
        assert!(res.members.contains(&children.1));
        assert_eq!(
            res.requests.len(),
            requests
                .iter()
                .filter(|r| r.user_uid() == children.0.uid())
                .collect::<Vec<_>>()
                .len(),
        );
        for request in
            requests.iter().filter(|r| r.user_uid() == children.0.uid())
        {
            assert!(res.requests.contains(request));
        }
        assert_eq!(res.transactions.len(), 1 * TRANSACTION_LIMIT);
        for transaction in transactions
            [transactions.len() - 1 * TRANSACTION_LIMIT..]
            .iter()
            .filter(|t| t.user_uid() == children.0.uid())
        {
            assert!(res.transactions.contains(transaction));
        }
    }

    #[actix_rt::test]
    async fn forbidden() {
        let pool = test_pool().await;
        let mut c = pool.acquire().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut c).unwrap();
        let other_family = create::family(&mut c, "Other Family");

        let err = execute(
            &mut pool.begin().await.unwrap(),
            FamilyConfiguration::new(db::entities::Currency::new(
                "TST".into(),
                CurrencyFormat::new("#{}"),
            )),
            State {
                user_uid: parent.uid().clone(),
                role: parent.role().clone(),
                family_uid: family.uid().clone(),
            },
            other_family.uid().clone(),
        )
        .await
        .err()
        .unwrap();

        assert_eq!(err, api::Error::forbidden("invalid family"));
    }
}
