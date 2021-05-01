use async_trait::async_trait;
use chrono::Datelike;

use crate::db;

pub struct Payer;

#[cfg(feature = "db_sqlite")]
const SQL: &'static str = "\
    INSERT INTO Transactions (transaction_type, user_uid, description, \
            amount, time)
        SELECT ?, user_uid, '', amount, ? \
        FROM Allowances
        WHERE schedule = ?";

#[async_trait]
impl super::Task for Payer {
    fn name(&self) -> &'static str {
        "allowance-payer"
    }

    async fn run(
        &self,
        transaction: &mut db::Transaction,
        timestamp: db::values::Timestamp,
    ) -> Result<(), super::Error> {
        Ok(sqlx::query(SQL)
            .bind(db::values::TransactionType::Allowance)
            .bind(timestamp)
            .bind(db::values::Schedule::from(timestamp.0.weekday()))
            .execute(transaction)
            .await
            .map(|_| ())?)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::prelude::*;

    use chrono::DateTime;

    use crate::db::entities::{Allowance, Entity, Transaction};
    use crate::db::test_pool;
    use crate::db::values::{TransactionType, UID};
    use crate::tasks::Task;

    use super::*;

    #[actix_rt::test]
    async fn run_simple() {
        let pool = test_pool().await;
        {
            let payer = Payer;
            let thursday =
                DateTime::parse_from_rfc3339("1970-01-01T01:00:00Z").unwrap();
            let friday =
                DateTime::parse_from_rfc3339("1970-01-02T01:00:00Z").unwrap();
            let allowance = Allowance::new(
                UID::new(),
                UID::new(),
                42,
                friday.weekday().into(),
            );

            // Create the allowance
            let mut connection = pool.acquire().await.unwrap();
            db::entities::allowance::tests::prepare(
                &mut connection,
                &allowance,
            )
            .await
            .unwrap();
            allowance.create(&mut connection).await.unwrap();

            // Run for a thursday
            let mut transaction = connection.begin().await.unwrap();
            {
                payer.run(&mut transaction, thursday.into()).await.unwrap();
                transaction.commit().await.unwrap();
            }
            assert_eq!(
                Transaction::list(&mut connection).await.unwrap(),
                Vec::new(),
            );

            // Run for a friday
            let mut transaction = connection.begin().await.unwrap();
            {
                payer.run(&mut transaction, friday.into()).await.unwrap();
                transaction.commit().await.unwrap();
            }
            let transactions =
                Transaction::list(&mut connection).await.unwrap();
            assert_eq!(transactions.len(), 1);
            assert_eq!(
                transactions[0].transaction_type(),
                &TransactionType::Allowance,
            );
            assert_eq!(transactions[0].user_uid(), allowance.user_uid());
            assert_eq!(transactions[0].amount(), &(*allowance.amount() as i64));
        }
    }
}
