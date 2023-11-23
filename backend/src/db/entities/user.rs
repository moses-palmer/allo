use crate::prelude::*;

use weru::database::{entity, sqlx};

use crate::db::values::{EmailAddress, Role, UID};

/// A description of a user.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[entity(Users)]
pub struct User {
    /// The unique identifier.
    pub uid: UID,

    /// The role of this user.
    pub role: Role,

    /// The user name.
    pub name: String,

    /// The user email address, if any validated email address exists.
    pub email: Option<EmailAddress>,

    /// The unique identifier of the family.
    pub family_uid: UID,
}

impl User {
    /// The SQL statement used to load all members of a family.
    const READ_BY_FAMILY: &'static str = sql_from_file!("User.read-by-family");

    /// Loads all members of a family.
    ///
    /// # Arguments
    /// *  `tx` - The database transaction.
    /// *  `family_uid` - The family UID.
    pub async fn read_by_family<'a>(
        tx: &mut Tx<'a>,
        family_uid: &UID,
    ) -> Result<Vec<Self>, DatabaseError> {
        sqlx::query_as(Self::READ_BY_FAMILY)
            .bind(family_uid)
            .fetch_all(tx.as_mut())
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
        prepare: |tx, e| {
            crate::db::entities::family::tests::entity_with_id(
                e.family_uid.clone(),
            ).create(tx.as_mut()).await
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
    async fn read_by_family() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();

        let family1 = create::family(&mut conn, "Family 1");
        let family2 = create::family(&mut conn, "Family 2");
        let user1 = create::user(
            &mut conn,
            Role::Parent,
            "User 1",
            "test1@example.com",
            &family1.uid,
        );
        let user2 = create::user(
            &mut conn,
            Role::Parent,
            "User 2",
            "test2@example.com",
            &family1.uid,
        );
        create::user(
            &mut conn,
            Role::Parent,
            "User 3",
            "test3@example.com",
            &family2.uid,
        );
        let mut tx = conn.begin().await.unwrap();

        let users = User::read_by_family(&mut tx, &user1.family_uid)
            .await
            .unwrap();
        assert_eq!(users.len(), 2);
        assert!(users.contains(&user1));
        assert!(users.contains(&user2));
    }
}
