use crate::prelude::*;

use weru::database::{entity, parameter};

use crate::db::values::{EmailAddress, PasswordHash, UID};

/// A description of a supported currency.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[entity(Passwords)]
pub struct Password {
    /// The unique identifier of the user.
    pub user_uid: UID,

    /// The stringified version of the hash.
    pub hash: PasswordHash,
}

impl Password {
    /// The SQL statement used to load a password hash by user email address.
    const READ_BY_EMAIL: &'static str = concat!(
        "SELECT user_uid, hash \
        FROM Passwords \
        LEFT JOIN Users \
            ON Passwords.user_uid = Users.uid \
        WHERE Users.email = ",
        parameter!(1)
    );

    /// Loads a password using the email address of the associated user.
    ///
    /// If no item corresponding to the keys exists, `Ok(None)` is
    /// returned.
    ///
    /// # Arguments
    /// *  `tx` - The database transaction.
    /// *  `email` - The email address.
    pub async fn read_by_email<'a>(
        tx: &mut Tx<'a>,
        email: &EmailAddress,
    ) -> Result<Option<Self>, DatabaseError> {
        sqlx::query_as(Self::READ_BY_EMAIL)
            .bind(email)
            .fetch_optional(tx.as_mut())
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
    use crate::db;
    use crate::db::entities::*;
    use crate::db::test_engine;
    use crate::db::values::Role;

    use super::*;

    #[actix_rt::test]
    async fn read_by_email() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let mut tx = conn.begin().await.unwrap();

        let email = "test@example.com".parse::<EmailAddress>().unwrap();
        let user = User::new(
            UID::new(),
            Role::Parent,
            "Test User".into(),
            Some(email.clone()),
            UID::new(),
        );
        db::entities::user::tests::prepare(&mut tx, &user)
            .await
            .unwrap();
        user.create(tx.as_mut()).await.unwrap();

        let password = Password {
            user_uid: user.uid.clone(),
            hash: PasswordHash::from_password("password123").unwrap(),
        };
        password.create(tx.as_mut()).await.unwrap();

        assert_eq!(
            Password::read_by_email(&mut tx, &email).await.unwrap(),
            Some(password),
        );
        assert!(Password::read_by_email(
            &mut tx,
            &"unknown@example.com".parse().unwrap()
        )
        .await
        .unwrap()
        .is_none());
    }
}
