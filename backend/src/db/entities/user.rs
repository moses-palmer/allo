use crate::db::values::{EmailAddress, Role, UID};

entity!(
    /// A description of a user.
    pub struct User in Users {
        /// The unique identifier.
        uid: UID,

        /// The role of this user.
        role: Role,

        /// The user name.
        name: String,

        /// The user email address, if any validated email address exists.
        email: Option<EmailAddress>,

        /// The unique identifier of the family.
        family_uid: UID,
    }
);

impl User {
    /// The SQL statement used to load all members of a family.
    const READ_BY_FAMILY: &'static str = concat!(
        "SELECT uid, role, name, email, family_uid \
        FROM Users \
        WHERE family_uid = ",
        parameter!(family_uid),
    );

    /// Loads all members of a family.
    ///
    /// # Arguments
    /// *  `e` - The database executor.
    /// *  `family_uid` - The family UID.
    pub async fn read_for_family<'a, E>(
        e: E,
        family_uid: &UID,
    ) -> Result<Vec<Self>, crate::db::Error>
    where
        E: ::sqlx::Executor<'a, Database = crate::db::Database>,
    {
        sqlx::query_as(Self::READ_BY_FAMILY)
            .bind(family_uid)
            .fetch_all(e)
            .await
    }
}

entity_tests! {
    User[UID = UID::new()] {
        entity: |id| User {
            uid: id,
            role: Role::Parent,
            name: "Test User".into(),
            email: None,
            family_uid: UID::new(),
        };
        modify: |e| User {
            name: "New Test User".into(),
            ..e
        };
        prepare: |c, e| {
            crate::db::entities::family::tests::entity_with_id(
                e.family_uid().clone(),
            ).create(c).await
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
    async fn read_for_family() {
        let pool = test_pool().await;
        {
            let mut c = pool.acquire().await.unwrap();

            let family1 = create::family(&mut c, "Family 1");
            let family2 = create::family(&mut c, "Family 2");
            let user1 = create::user(
                &mut c,
                Role::Parent,
                "User 1",
                "test1@example.com",
                family1.uid(),
            );
            let user2 = create::user(
                &mut c,
                Role::Parent,
                "User 2",
                "test2@example.com",
                family1.uid(),
            );
            create::user(
                &mut c,
                Role::Parent,
                "User 3",
                "test3@example.com",
                family2.uid(),
            );

            let users = User::read_for_family(&mut c, user1.family_uid())
                .await
                .unwrap();
            assert_eq!(users.len(), 2);
            assert!(users.contains(&user1));
            assert!(users.contains(&user2));
        }
    }
}
