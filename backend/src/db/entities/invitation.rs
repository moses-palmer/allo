use crate::db::entities::allowance::Description as AllowanceDescription;
use crate::db::entities::user::Description as UserDescription;
use crate::db::values::{EmailAddress, Role, Schedule, Timestamp, UID};

entity!(
    /// A description of an invited user.
    pub struct Invitation in Invitations {
        /// The unique identifier.
        uid: UID,

        /// The future role of this user.
        role: Role,

        /// The user name.
        name: String,

        /// The future user email address.
        email: EmailAddress,

        /// The allowance amount, if this user is a child.
        allowance_amount: Option<u32>,

        /// The schedule of the allowance, if this user is a child.
        allowance_schedule: Option<Schedule>,

        /// The creation timestamp.
        time: Timestamp,

        /// The unique identifier of the family.
        family_uid: UID,
    }
);

impl Invitation {
    /// The SQL statement used to load all members of a family.
    const READ_BY_FAMILY: &'static str = concat!(
        "SELECT uid, role, name, email, allowance_amount, allowance_schedule, \
            time, family_uid \
        FROM Invitations \
        WHERE family_uid = ",
        parameter!(family_uid),
    );

    /// Loads all invitations for a family.
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
            let invitation1 = create::invitation(
                &mut c,
                Role::Parent,
                "User 1",
                "test1@example.com",
                family1.uid(),
            );
            let invitation2 = create::invitation(
                &mut c,
                Role::Parent,
                "User 2",
                "test2@example.com",
                family1.uid(),
            );
            create::invitation(
                &mut c,
                Role::Parent,
                "User 3",
                "test3@example.com",
                family2.uid(),
            );

            let invitations =
                Invitation::read_for_family(&mut c, invitation1.family_uid())
                    .await
                    .unwrap();
            assert_eq!(invitations.len(), 2);
            assert!(invitations.contains(&invitation1));
            assert!(invitations.contains(&invitation2));
        }
    }
}
