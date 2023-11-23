use weru::database::entity;

use crate::db::values::UID;

#[derive(Clone, Debug, PartialEq)]
#[entity(Configurations)]
/// The configuration used by a family.
pub struct Configuration {
    /// The family using this configuration.
    family_uid: UID,

    /// The currency name.
    currency: String,
}

entity_tests! {
    Configuration[UID = UID::new()] {
        entity: |id| Configuration {
            family_uid: UID::new(),
            currency: "TST".into(),
        };
        modify: |e| Configuration {
            ..e
        };
        prepare: |tx, e| {
            crate::db::entities::family::tests::entity_with_id(
                e.family_uid.clone(),
            ).create(tx.as_mut()).await?;
            crate::db::entities::currency::tests::entity_with_id(
                e.currency.clone(),
            ).create(tx.as_mut()).await
        };
    }
}
