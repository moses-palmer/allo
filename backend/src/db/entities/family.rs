use crate::db::values::UID;

entity!(
    /// A description of a supported currency.
    pub struct Family in Families {
        /// The unique identifier.
        uid: UID,

        /// The family name.
        name: String,
    }
);

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
