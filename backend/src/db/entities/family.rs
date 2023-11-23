use crate::prelude::*;

use weru::database::entity;

use crate::db::values::UID;

/// A description of a supported currency.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[entity(Families)]
pub struct Family {
    /// The unique identifier.
    pub uid: UID,

    /// The family name.
    pub name: String,
}

entity_tests! {
    Family[UID = UID::new()] {
        entity: |id| Family {
            uid: id,
            name: "Test Family".into(),
        };
        modify: |e| Family {
            name: "New Family Name".into(),
            ..e
        };
        prepare: |_c, _e| {
            Ok(())
        };
    }
}
