use crate::prelude::*;

use chrono::Datelike;
use weru::async_trait::async_trait;

use crate::db;

pub struct AllowancePayer;

impl AllowancePayer {
    const PAY: &'static str = sql_from_file!("Allowance.pay");
}

#[async_trait]
impl super::Task for AllowancePayer {
    fn name(&self) -> &'static str {
        "allowance-payer"
    }

    async fn run<'a>(
        &self,
        tx: &mut Tx<'a>,
        timestamp: db::values::Timestamp,
    ) -> Result<(), super::Error> {
        Ok(sqlx::query(Self::PAY)
            .bind(db::values::TransactionType::Allowance)
            .bind(timestamp)
            .bind(db::values::Schedule::from(timestamp.0.weekday()))
            .execute(tx.as_mut())
            .await
            .map(|_| ())?)
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use weru::database::Entity;

    use crate::db::entities::{Allowance, Transaction};
    use crate::db::test_engine;
    use crate::db::values::{TransactionType, UID};
    use crate::tasks::Task;

    use super::*;

    #[actix_rt::test]
    async fn run_simple() {
        let database = test_engine().await;
        let payer = AllowancePayer;
        let thursday =
            DateTime::parse_from_rfc3339("1970-01-01T01:00:00Z").unwrap();
        let friday =
            DateTime::parse_from_rfc3339("1970-01-02T01:00:00Z").unwrap();
        let allowance =
            Allowance::new(UID::new(), UID::new(), 42, friday.weekday().into());

        // Create the allowance
        let mut conn = database.connection().await.unwrap();
        {
            let mut tx = conn.begin().await.unwrap();
            db::entities::allowance::tests::prepare(&mut tx, &allowance)
                .await
                .unwrap();
            allowance.create(tx.as_mut()).await.unwrap();
            tx.commit().await.unwrap();
        }

        // Run for a thursday
        {
            let mut tx = conn.begin().await.unwrap();
            payer.run(&mut tx, thursday.into()).await.unwrap();
            tx.commit().await.unwrap();
        }
        assert_eq!(Transaction::list(conn.as_mut()).await.unwrap(), Vec::new());

        // Run for a friday
        {
            let mut tx = conn.begin().await.unwrap();
            payer.run(&mut tx, friday.into()).await.unwrap();
            tx.commit().await.unwrap();
        }

        let mut tx = conn.begin().await.unwrap();
        let transactions = Transaction::list(tx.as_mut()).await.unwrap();
        assert_eq!(transactions.len(), 1);
        assert_eq!(
            transactions[0].transaction_type,
            TransactionType::Allowance,
        );
        assert_eq!(transactions[0].user_uid, allowance.user_uid);
        assert_eq!(transactions[0].amount, allowance.amount as i64);
    }
}
