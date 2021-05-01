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
