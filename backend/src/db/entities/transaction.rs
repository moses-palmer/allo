use crate::prelude::*;

use std::ops::Range;

use either::Either;
use weru::database::entity;
use weru::futures::StreamExt;

use crate::db::values::{Timestamp, TransactionType, UID};

/// A description of a transaction.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[entity(Transactions)]
pub struct Transaction {
    /// The unique identifier.
    ///
    /// We want these to be generated by the database, so we use a plain
    /// integer.
    pub uid: i64,

    /// The type of transaction.
    pub transaction_type: TransactionType,

    /// The user involved in this transaction.
    pub user_uid: UID,

    /// A description.
    pub description: String,

    /// The amount.
    pub amount: i64,

    /// The timestamp of this transaction.
    pub time: Timestamp,
}

impl Transaction {
    /// The SQL statement used to create a transaction with an automatic UID.
    const CREATE_WITH_AUTO_UID: &'static str =
        sql_from_file!("Transaction.create-with-auth-id");

    /// The SQL statement used to load transactions for a user.
    const READ_FOR_USER_LIMIT: &'static str =
        sql_from_file!("Transaction.read-for-user-limit");

    /// The SQL statement used to load the balace for a user.
    const BALANCE: &'static str = sql_from_file!("Transaction.balance");

    /// Creates a transaction in the database, delegating selection of UID.
    ///
    /// # Arguments
    /// *  `tx` - The database transaction.
    /// *  `transaction_type` - The transaction type.
    /// *  `user_uid` - The user UID.
    /// *  `description` - A description.
    /// *  `amount` - The amount. Negative amounts are withdrawals.
    /// *  `time` - The timestamp of the transaction.
    pub async fn create_with_auto_uid<'a>(
        tx: &mut Tx<'a>,
        transaction_type: TransactionType,
        user_uid: UID,
        description: String,
        amount: i64,
        time: Timestamp,
    ) -> Result<Self, DatabaseError> {
        let mut stream = sqlx::query(Self::CREATE_WITH_AUTO_UID)
            .bind(transaction_type)
            .bind(user_uid.clone())
            .bind(description.clone())
            .bind(amount)
            .bind(time)
            .fetch_many(tx.as_mut());
        while let Some(e) = stream.next().await {
            if let Either::Right(row) = e? {
                let uid = row.get::<<Self as Entity>::Key, _>(0);
                return Ok(Self {
                    uid,
                    transaction_type,
                    user_uid,
                    description,
                    amount,
                    time,
                });
            }
        }

        Err(DatabaseError::RowNotFound)
    }

    /// Loads transactions for a user.
    ///
    /// # Arguments
    /// *  `tx` - The database transaction.
    /// *  `user_uid` - The user UID.
    /// *  `limit` - The maximum number of transactions to retrieve.
    pub async fn read_for_user_limit<'a>(
        tx: &mut Tx<'a>,
        user_uid: &UID,
        range: Range<usize>,
    ) -> Result<Vec<Self>, DatabaseError> {
        Ok(sqlx::query_as(Self::READ_FOR_USER_LIMIT)
            .bind(user_uid)
            .bind((range.end - range.start) as u32)
            .bind(range.start as u32)
            .fetch_all(tx.as_mut())
            .await?
            .into_iter()
            .rev()
            .collect())
    }

    /// Loads the balance for a user.
    ///
    /// # Arguments
    /// *  `tx` - The database transaction.
    /// *  `user_uid` - The user UID.
    pub async fn balance<'a>(
        tx: &mut Tx<'a>,
        user_uid: &UID,
    ) -> Result<Option<i64>, DatabaseError> {
        Ok(sqlx::query(Self::BALANCE)
            .bind(user_uid)
            .fetch_optional(tx.as_mut())
            .await?
            .map(|r| r.get(0)))
    }
}

entity_tests! {
    Transaction[i64 = i64::default()] {
        entity: |id| Transaction {
            uid: id,
            transaction_type: TransactionType::Gift,
            user_uid: UID::new(),
            description: "description".into(),
            amount: 42,
            time: Timestamp::now(),
        };
        modify: |e| Transaction {
            description: "another description".into(),
            ..e
        };
        prepare: |tx, e| {
            let u = crate::db::entities::user::tests::entity_with_id(
                e.user_uid.clone(),
            );
            crate::db::entities::user::tests::prepare(tx, &u).await?;
            u.create(tx.as_mut()).await
        };
    }
}

#[cfg(test)]
mod impl_tests {
    use std::time::Duration;

    use weru::database::Entity;

    use crate::db::entities::create;
    use crate::db::test_engine;
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn create_with_auto_uid() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let family = create::family(&mut conn, "Family");
        let user = create::user(
            &mut conn,
            Role::Parent,
            "User 1",
            "test1@example.com",
            &family.uid,
        );
        let mut tx = conn.begin().await.unwrap();

        let transaction = Transaction::create_with_auto_uid(
            &mut tx,
            TransactionType::Gift,
            user.uid.clone(),
            "description".into(),
            42,
            Timestamp::now(),
        )
        .await
        .unwrap();

        assert_eq!(
            Some(&transaction),
            Transaction::read(tx.as_mut(), &transaction.uid)
                .await
                .unwrap()
                .as_ref(),
        );
    }

    #[actix_rt::test]
    async fn read_for_user_limit() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let family = create::family(&mut conn, "Family");
        let user1 = create::user(
            &mut conn,
            Role::Parent,
            "User 1",
            "test1@example.com",
            &family.uid,
        );
        let user2 = create::user(
            &mut conn,
            Role::Parent,
            "User 2",
            "test2@example.com",
            &family.uid,
        );
        let start = Timestamp::now();
        let all_transactions = (0..40)
            .map(|i| {
                create::transaction(
                    &mut conn,
                    TransactionType::Gift,
                    if i & 1 != 0 { &user1.uid } else { &user2.uid },
                    &format!("description{}", i),
                    (i + 1) * 3,
                    start
                        .0
                        .checked_add_signed(
                            chrono::Duration::from_std(Duration::from_secs(
                                i as u64,
                            ))
                            .unwrap(),
                        )
                        .unwrap()
                        .into(),
                )
            })
            .collect::<Vec<_>>();
        let mut tx = conn.begin().await.unwrap();

        let transactions =
            Transaction::read_for_user_limit(&mut tx, &user1.uid, 0..10)
                .await
                .unwrap();
        assert_eq!(
            transactions,
            all_transactions
                .iter()
                .filter(|t| t.user_uid == user1.uid)
                .skip(10)
                .cloned()
                .collect::<Vec<_>>(),
        );
        let transactions =
            Transaction::read_for_user_limit(&mut tx, &user1.uid, 1..10)
                .await
                .unwrap();
        assert_eq!(
            transactions,
            all_transactions
                .iter()
                .filter(|t| t.user_uid == user1.uid)
                .skip(10)
                .take(9)
                .cloned()
                .collect::<Vec<_>>(),
        );
    }

    #[actix_rt::test]
    async fn balance() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let family = create::family(&mut conn, "Family");
        let user1 = create::user(
            &mut conn,
            Role::Parent,
            "User 1",
            "test1@example.com",
            &family.uid,
        );
        let user2 = create::user(
            &mut conn,
            Role::Parent,
            "User 2",
            "test2@example.com",
            &family.uid,
        );

        let start = Timestamp::now();
        (0..40).for_each(|i| {
            create::transaction(
                &mut conn,
                TransactionType::Gift,
                if i & 1 != 0 { &user1.uid } else { &user2.uid },
                &format!("description{}", i),
                (i + 1) * 3,
                start
                    .0
                    .checked_add_signed(
                        chrono::Duration::from_std(Duration::from_secs(
                            i as u64,
                        ))
                        .unwrap(),
                    )
                    .unwrap()
                    .into(),
            );
        });
        let mut tx = conn.begin().await.unwrap();

        assert_eq!(
            Transaction::balance(&mut tx, &user1.uid).await.unwrap(),
            Some((0..40).filter(|i| i & 1 != 0).map(|i| (i + 1) * 3).sum()),
        );
    }
}
