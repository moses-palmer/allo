use crate::db::values::{PasswordHash, UID};

use super::Entity;

entity!(
    /// A description of a supported currency.
    pub struct Password in Passwords {
        /// The unique identifier of the user.
        user_uid: UID,

        /// The stringified version of the hash.
        hash: PasswordHash,
    }
);

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
