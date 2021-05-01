use crate::db::values::{EmailAddress, PasswordHash, UID};

entity!(
    /// A description of a supported currency.
    pub struct Password in Passwords {
        /// The unique identifier of the user.
        user_uid: UID,

        /// The stringified version of the hash.
        hash: PasswordHash,
    }
);

impl Password {
    /// The SQL statement used to load a password hash by user email address.
    const READ_BY_EMAIL: &'static str = concat!(
        "SELECT user_uid, hash \
        FROM Passwords \
        LEFT JOIN Users \
            ON Passwords.user_uid = Users.uid \
        WHERE Users.email = ",
        parameter!(email)
    );

    /// Loads a password using the email address of the associated user.
    ///
    /// If no item corresponding to the keys exists, `Ok(None)` is
    /// returned.
    ///
    /// # Arguments
    /// *  `e` - The database executor.
    /// *  `email` - The email address.
    pub async fn read_by_email<'a, E>(
        e: E,
        email: &EmailAddress,
    ) -> Result<Option<Self>, crate::db::Error>
    where
        E: ::sqlx::Executor<'a, Database = crate::db::Database>,
    {
        sqlx::query_as(Self::READ_BY_EMAIL)
            .bind(email)
            .fetch_optional(e)
            .await
    }
}

entity_tests! {
    Password[UID = UID::new()] {
        entity: |id| Password {
            user_uid: id,
            hash: PasswordHash::from_password("password").unwrap(),
        };
        modify: |e| Password {
            hash: PasswordHash::from_password("secret").unwrap(),
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

    use crate::db;
    use crate::db::entities::{Entity, User};
    use crate::db::test_pool;
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn read_by_email() {
        let pool = test_pool().await;
        {
            let mut connection = pool.acquire().await.unwrap();
            let email = "test@example.com".parse::<EmailAddress>().unwrap();
            let user = User::new(
                UID::new(),
                Role::Parent,
                "Test User".into(),
                Some(email.clone()),
                UID::new(),
            );
            db::entities::user::tests::prepare(&mut connection, &user)
                .await
                .unwrap();
            user.create(&mut connection).await.unwrap();

            let password = Password {
                user_uid: user.uid().clone(),
                hash: PasswordHash::from_password("password123").unwrap(),
            };
            password.create(&mut connection).await.unwrap();

            assert_eq!(
                Password::read_by_email(&mut connection, &email)
                    .await
                    .unwrap(),
                Some(password),
            );
            assert!(Password::read_by_email(
                &mut connection,
                &"unknown@example.com".parse().unwrap()
            )
            .await
            .unwrap()
            .is_none());
        }
    }
}
