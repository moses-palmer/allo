use crate::prelude::*;

use std::collections::HashMap;

use crate::api;
use crate::api::session::State;
use crate::configuration::FamilyConfiguration;
use crate::db;
use crate::db::entities::{Family, Invitation, Request, Transaction, User};
use crate::db::values::{Role, UID};

/// The maximum number of transactions to return per user.
const TRANSACTION_LIMIT: usize = 5;

/// Retrieves an overview of the account.
#[get("overview/{family_uid}")]
pub async fn handle(
    defaults: web::Data<FamilyConfiguration>,
    database: web::Data<DatabaseEngine>,
    session: Session,
    family_uid: web::Path<UID>,
) -> impl Responder {
    let mut conn = database.connection().await?;
    let mut tx = conn.begin().await?;
    let state = State::load(&session)?;
    {
        let res = execute(
            &mut tx,
            (*defaults.into_inner()).clone(),
            state,
            &family_uid,
        )
        .await?;
        tx.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    tx: &mut Tx<'a>,
    defaults: FamilyConfiguration,
    state: State,
    family_uid: &UID,
) -> Result<Res, api::Error> {
    let State { user_uid, role, .. } = state.assert_family(family_uid)?;
    let configuration = FamilyConfiguration::read(&mut *tx, family_uid)
        .await?
        .unwrap_or(defaults);
    let family = api::expect(Family::read(tx.as_mut(), family_uid).await?)?;
    let members = User::read_for_family(tx, &family_uid).await?;
    let invitations = Invitation::read_for_family(tx, family_uid).await?;
    let requests = match role {
        Role::Parent => Request::read_for_family(tx, family_uid).await?,
        Role::Child => Request::read_for_user(tx, &user_uid).await?,
    };
    let children = || {
        members.iter().filter(|user| match (role, user.role) {
            (Role::Parent, Role::Child) => true,
            (Role::Child, _) => &user.uid == &user_uid,
            _ => false,
        })
    };
    let transactions = {
        let mut transactions = Vec::new();
        for child in children() {
            transactions.extend(
                Transaction::read_for_user_limit(
                    tx,
                    &child.uid,
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
                child.uid.clone(),
                Transaction::balance(tx, &child.uid).await?.unwrap_or(0),
            );
        }
        balances
    };

    Ok(Res {
        currency: configuration.currency().clone(),
        family,
        members,
        invitations,
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

    /// All pending invitations for this family.
    invitations: Vec<Invitation>,

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
    use crate::db::test_engine;
    use crate::db::values::CurrencyFormat;

    use super::*;

    #[actix_rt::test]
    async fn success_parent() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, children, transactions, requests) =
            tests::populate(&mut conn).unwrap();
        let invitations = vec![
            create::invitation(
                &mut conn,
                Role::Parent,
                "User 1",
                "test1@example.com",
                &family.uid,
            ),
            create::invitation(
                &mut conn,
                Role::Parent,
                "User 2",
                "test2@example.com",
                &family.uid,
            ),
        ];

        let res = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                FamilyConfiguration::new(db::entities::Currency::new(
                    "TST".into(),
                    CurrencyFormat::new("#{}"),
                )),
                State {
                    user_uid: parent.uid.clone(),
                    role: parent.role.clone(),
                    family_uid: family.uid.clone(),
                },
                &family.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(res.family, family);
        assert_eq!(res.members.len(), 3);
        assert!(res.members.contains(&parent));
        assert!(res.members.contains(&children.0));
        assert!(res.members.contains(&children.1));
        assert_eq!(res.invitations.len(), invitations.len());
        assert!(res.invitations.contains(&invitations[0]));
        assert!(res.invitations.contains(&invitations[1]));
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
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, children, transactions, requests) =
            tests::populate(&mut conn).unwrap();

        let res = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                FamilyConfiguration::new(db::entities::Currency::new(
                    "TST".into(),
                    CurrencyFormat::new("#{}"),
                )),
                State {
                    user_uid: children.0.uid.clone(),
                    role: children.0.role.clone(),
                    family_uid: family.uid.clone(),
                },
                &family.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(res.family, family);
        assert_eq!(res.members.len(), 3);
        assert!(res.members.contains(&parent));
        assert!(res.members.contains(&children.0));
        assert!(res.members.contains(&children.1));
        assert_eq!(
            res.requests.len(),
            requests
                .iter()
                .filter(|r| r.user_uid == children.0.uid)
                .collect::<Vec<_>>()
                .len(),
        );
        for request in requests.iter().filter(|r| r.user_uid == children.0.uid)
        {
            assert!(res.requests.contains(request));
        }
        assert_eq!(res.transactions.len(), 1 * TRANSACTION_LIMIT);
        for transaction in transactions
            [transactions.len() - 1 * TRANSACTION_LIMIT..]
            .iter()
            .filter(|t| t.user_uid == children.0.uid)
        {
            assert!(res.transactions.contains(transaction));
        }
    }

    #[actix_rt::test]
    async fn forbidden() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (family, parent, _, _, _) = tests::populate(&mut conn).unwrap();
        let other_family = create::family(&mut conn, "Other Family");

        let err = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                FamilyConfiguration::new(db::entities::Currency::new(
                    "TST".into(),
                    CurrencyFormat::new("#{}"),
                )),
                State {
                    user_uid: parent.uid.clone(),
                    role: parent.role.clone(),
                    family_uid: family.uid.clone(),
                },
                &other_family.uid,
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(err, api::Error::forbidden("invalid family"));
    }
}
