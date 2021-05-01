use crate::db::values::{Schedule, UID};

entity!(
    /// The allowance for a user.
    pub struct Allowance in Allowances {
        /// The unique identifier.
        uid: UID,

        /// The user receiving this allowance.
        user_uid: UID,

        /// The amount.
        amount: u32,

        /// The schedule of the allowance.
        schedule: Schedule,
    }
);

impl Allowance {
    /// The SQL statement used to load all allowances from a user.
    const READ_FOR_USER: &'static str = concat!(
        "SELECT uid, user_uid, amount, schedule \
        FROM Allowances \
        WHERE user_uid = ",
        parameter!(user_uid),
    );

    /// Loads all allowances for a user.
    ///
    /// # Arguments
    /// *  `e` - The database executor.
    /// *  `user_uid` - The user UID.
    pub async fn read_for_user<'a, E>(
        e: E,
        user_uid: &UID,
    ) -> Result<Vec<Self>, crate::db::Error>
    where
        E: ::sqlx::Executor<'a, Database = crate::db::Database>,
    {
        sqlx::query_as(Self::READ_FOR_USER)
            .bind(user_uid)
            .fetch_all(e)
            .await
    }
}

entity_tests! {
    Allowance[UID = UID::new()] {
        entity: |id| Allowance {
            uid: id,
            user_uid: UID::new(),
            amount: 42,
            schedule: "mon".parse::<Schedule>().unwrap(),
        };
        modify: |e| Allowance {
            schedule: "tue".parse::<Schedule>().unwrap(),
            ..e
        };
        prepare: |c, e| {
            let u = crate::db::entities::user::tests::entity_with_id(
                e.user_uid().clone(),
            );
            crate::db::entities::user::tests::prepare(c, &u).await?;
            u.create(c).await
        };
    }
}

#[cfg(test)]
mod impl_tests {
    use actix_rt;

    use crate::db::entities::create;
    use crate::db::test_pool;
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn read_for_user() {
        let pool = test_pool().await;
        {
            let mut c = pool.acquire().await.unwrap();

            let family = create::family(&mut c, "Family");
            let user1 = create::user(
                &mut c,
                Role::Parent,
                "User 1",
                "test1@example.com",
                family.uid(),
            );
            let user2 = create::user(
                &mut c,
                Role::Parent,
                "User 2",
                "test2@example.com",
                family.uid(),
            );
            let allowance1 = create::allowance(
                &mut c,
                user1.uid(),
                42,
                "mon".parse().unwrap(),
            );
            let allowance2 = create::allowance(
                &mut c,
                user1.uid(),
                43,
                "tue".parse().unwrap(),
            );
            create::allowance(&mut c, user2.uid(), 44, "wed".parse().unwrap());

            let allowances =
                Allowance::read_for_user(&mut c, allowance1.user_uid())
                    .await
                    .unwrap();
            assert_eq!(allowances.len(), 2);
            assert!(allowances.contains(&allowance1));
            assert!(allowances.contains(&allowance2));
        }
    }
}
