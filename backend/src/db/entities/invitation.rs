use crate::prelude::*;

use weru::database::entity;

use crate::db::entities::allowance::AllowanceDescription;
use crate::db::entities::user::UserDescription;
use crate::db::values::{EmailAddress, Role, Schedule, Timestamp, UID};

/// A description of an invited user.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[entity(Invitations)]
pub struct Invitation {
    /// The unique identifier.
    pub uid: UID,

    /// The future role of this user.
    pub role: Role,

    /// The user name.
    pub name: String,

    /// The future user email address.
    pub email: EmailAddress,

    /// The allowance amount, if this user is a child.
    pub allowance_amount: Option<u32>,

    /// The schedule of the allowance, if this user is a child.
    pub allowance_schedule: Option<Schedule>,

    /// The creation timestamp.
    pub time: Timestamp,

    /// The unique identifier of the family.
    pub family_uid: UID,
}

impl Invitation {
    /// The SQL statement used to load all members of a family.
    const READ_BY_FAMILY: &'static str =
        sql_from_file!("Invitation.read-by-family");

    /// Loads all invitations for a family.
    ///
    /// # Arguments
    /// *  `tx` - The database transaction.
    /// *  `family_uid` - The family UID.
    pub async fn read_for_family<'a>(
        tx: &mut Tx<'a>,
        family_uid: &UID,
    ) -> Result<Vec<Self>, DatabaseError> {
        sqlx::query_as(Self::READ_BY_FAMILY)
            .bind(family_uid)
            .fetch_all(tx.as_mut())
            .await
    }

    /// Creates an allowance description from this invitation.
    ///
    /// If the invitation is for a parent, this method will return nothing.
    pub fn allowance(&self) -> Option<AllowanceDescription> {
        if self.role == Role::Child {
            let amount = self.allowance_amount?;
            let schedule = self.allowance_schedule.clone()?;
            Some(AllowanceDescription {
                amount: Some(amount),
                schedule: Some(schedule),
                ..Default::default()
            })
        } else {
            None
        }
    }

    /// Creates a user description from this invitation.
    pub fn user(&self) -> UserDescription {
        UserDescription {
            role: Some(self.role),
            name: Some(self.name.clone()),
            email: Some(Some(self.email.clone())),
            family_uid: Some(self.family_uid.clone()),
        }
    }
}

entity_tests! {
    Invitation[UID = UID::new()] {
        entity: |id| Invitation {
            uid: id,
            role: Role::Parent,
            name: "Test User".into(),
            email: "test@email.com".parse().unwrap(),
            allowance_amount: None,
            allowance_schedule: None,
            time: Timestamp::now(),
            family_uid: UID::new(),
        };
        modify: |e| Invitation {
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
    async fn read_for_family() {
        let database = test_engine().await;
        let mut conn = database.connection().await.unwrap();
        let family1 = create::family(&mut conn, "Family 1");
        let family2 = create::family(&mut conn, "Family 2");
        let invitation1 = create::invitation(
            &mut conn,
            Role::Parent,
            "User 1",
            "test1@example.com",
            &family1.uid,
        );
        let invitation2 = create::invitation(
            &mut conn,
            Role::Parent,
            "User 2",
            "test2@example.com",
            &family1.uid,
        );
        create::invitation(
            &mut conn,
            Role::Parent,
            "User 3",
            "test3@example.com",
            &family2.uid,
        );
        let mut tx = conn.begin().await.unwrap();

        let invitations =
            Invitation::read_for_family(&mut tx, &invitation1.family_uid)
                .await
                .unwrap();
        assert_eq!(invitations.len(), 2);
        assert!(invitations.contains(&invitation1));
        assert!(invitations.contains(&invitation2));
    }
}
