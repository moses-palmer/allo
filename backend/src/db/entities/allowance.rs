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
