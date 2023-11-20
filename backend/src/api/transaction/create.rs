use crate::prelude::*;

use crate::api;
use crate::api::session::State;
use crate::db::entities::{transaction, Transaction, User};
use crate::db::values::{Role, Timestamp, UID};

/// Generates a transaction.
#[post("transaction/{user_uid}")]
pub async fn handle(
    database: web::Data<DatabaseEngine>,
    session: Session,
    req: web::Json<Req>,
    user_uid: web::Path<UID>,
) -> impl Responder {
    let mut conn = database.connection().await?;
    let mut tx = conn.begin().await?;
    let state = State::load(&session)?;
    {
        let res =
            execute(&mut tx, state, &req.into_inner(), &user_uid.into_inner())
                .await?;
        tx.commit().await?;
        api::ok(res)
    }
}

pub async fn execute<'a>(
    tx: &mut Tx<'a>,
    state: State,
    req: &Req,
    user_uid: &UID,
) -> Result<Res, api::Error> {
    let user = User::read(tx.as_mut(), user_uid)
        .await?
        .ok_or_else(|| api::Error::not_found("unknown request"))?;
    state
        .assert_role(Role::Parent)?
        .assert_family(&user.family_uid)?;

    let transaction = Transaction::create_with_auto_uid(
        tx,
        api::argument(req.transaction_type)?,
        user_uid.clone(),
        api::argument(req.description.clone())?,
        api::argument(req.amount)?,
        Timestamp::now(),
    )
    .await?;

    Ok(Res { transaction })
}

pub type Req = transaction::TransactionDescription;

#[derive(Deserialize, Serialize)]
pub struct Res {
    /// The generated transaction.
    pub transaction: Transaction,
}

#[cfg(test)]
mod tests {
    use crate::api::tests;
    use crate::db::test_engine;
    use crate::db::values::TransactionType;

    use super::*;

    #[actix_rt::test]
    async fn success() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let (_, parent, children, _, _) = tests::populate(&mut conn).unwrap();
        let amount = 424242;

        let res = {
            let mut tx = conn.begin().await.unwrap();
            let r = execute(
                &mut tx,
                State {
                    user_uid: parent.uid.clone(),
                    role: parent.role.clone(),
                    family_uid: parent.family_uid.clone(),
                },
                &Req {
                    transaction_type: Some(TransactionType::Gift),
                    description: Some("A description!".into()),
                    amount: Some(amount),
                    ..Default::default()
                },
                &children.0.uid,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(res.transaction.transaction_type, TransactionType::Gift);
        assert_eq!(res.transaction.description, String::from("A description!"));
        assert_eq!(res.transaction.amount, amount);
        assert_eq!(
            Transaction::read(conn.as_mut(), &res.transaction.uid)
                .await
                .unwrap(),
            Some(res.transaction),
        );
    }

    #[actix_rt::test]
    async fn forbidden() {
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
                &Req {
                    transaction_type: Some(TransactionType::Gift),
                    description: Some("A description!".into()),
                    amount: Some(10),
                    ..Default::default()
                },
                &children.0.uid,
            )
            .await
            .err()
            .unwrap();
            tx.commit().await.unwrap();
            r
        };

        assert_eq!(err, api::Error::forbidden("invalid role"));
    }
}
