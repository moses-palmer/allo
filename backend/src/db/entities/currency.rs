use crate::db::values::CurrencyFormat;

entity!(
    /// A description of a supported currency.
    pub struct Currency in Currencies {
        /// The currency name.
        name: String,

        /// The format string used to stringify values in this currency.
        format: CurrencyFormat,
    }
);

entity_tests! {
    Currency[String = String::from("TST")] {
        entity: |id| Currency {
            name: id,
            format: CurrencyFormat::new("#{}"),
        };
        modify: |e| Currency {
            format: CurrencyFormat::new("%{}"),
            ..e
        };
        prepare: |_c, _e| {
            Ok(())
        };
    }
}
