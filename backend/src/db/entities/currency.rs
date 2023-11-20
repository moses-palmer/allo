use crate::prelude::*;

use weru::database::entity;

use crate::db::values::CurrencyFormat;

/// A description of a supported currency.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[entity(Currencies)]
pub struct Currency {
    /// The currency name.
    name: String,

    /// The format string used to stringify values in this currency.
    format: CurrencyFormat,
}

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
