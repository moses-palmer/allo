use crate::prelude::*;

use weru::database::entity;

use crate::db::values::{Schedule, UID};

/// The allowance for a user.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[entity(Allowances)]
pub struct Allowance {
    /// The unique identifier.
    pub uid: UID,

    /// The user receiving this allowance.
    pub user_uid: UID,

    /// The amount.
    pub amount: u32,

    /// The schedule of the allowance.
    pub schedule: Schedule,
}

impl Allowance {
    /// The SQL statement used to load all allowances from a user.
    const READ_FOR_USER: &'static str =
        sql_from_file!("Allowance.read-for-user");

    /// Loads all allowances for a user.
    ///
    /// # Arguments
    /// *  `tx` - The database transaction.
    /// *  `user_uid` - The user UID.
    pub async fn read_for_user<'a>(
        tx: &mut Tx<'a>,
        user_uid: &UID,
    ) -> Result<Vec<Self>, DatabaseError> {
        sqlx::query_as(Self::READ_FOR_USER)
            .bind(user_uid)
            .fetch_all(tx.as_mut())
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
    use actix_rt;

    use crate::db::entities::create;
    use crate::db::test_engine;
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn read_for_user() {
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
        let allowance1 = create::allowance(
            &mut conn,
            &user1.uid,
            42,
            "mon".parse().unwrap(),
        );
        let allowance2 = create::allowance(
            &mut conn,
            &user1.uid,
            43,
            "tue".parse().unwrap(),
        );
        create::allowance(&mut conn, &user2.uid, 44, "wed".parse().unwrap());
        let mut tx = conn.begin().await.unwrap();

        let allowances =
            Allowance::read_for_user(&mut tx, &allowance1.user_uid)
                .await
                .unwrap();
        assert_eq!(allowances.len(), 2);
        assert!(allowances.contains(&allowance1));
        assert!(allowances.contains(&allowance2));
    }
}
