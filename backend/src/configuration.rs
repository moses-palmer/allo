use crate::prelude::*;

use std::fs;
use std::io;

use weru::database::parameter;
use weru::toml;

use crate::db::entities::Currency;
use crate::db::values::UID;

#[derive(Clone, Deserialize, Serialize)]
pub struct Configuration {
    /// Server related configurations.
    pub server: Server,

    /// Session related configurations.
    pub session: SessionConfiguration,

    /// Database connection information.
    pub database: DatabaseConfiguration,

    /// The configuration for the notifier.
    pub channel: ChannelConfiguration,

    /// The configuration for the email sender.
    pub email: EMailConfiguration,

    /// The default configuration to apply to families.
    pub defaults: FamilyConfiguration,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Server {
    /// The external URL for the frontend application.
    pub url: String,

    /// The bind string.
    pub bind: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct FamilyConfiguration {
    /// The currency used by this family.
    pub currency: Currency,
}

impl Configuration {
    /// Loads the application configuration from a TOML file.
    ///
    /// # Arguments
    /// *  `path` - The path to the configuration file.
    pub fn load(path: &str) -> io::Result<Self> {
        toml::from_str(&fs::read_to_string(path)?)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    /// The bind string to which to listen.
    pub fn server_bind(&self) -> String {
        self.server.bind.clone()
    }

    /// The default configuration.
    pub fn defaults(&self) -> FamilyConfiguration {
        self.defaults.clone()
    }
}

impl FamilyConfiguration {
    const READ: &'static str = concat!(
        "SELECT family_uid, Currencies.name as name, \
        Currencies.format as format
        FROM Configurations \
        LEFT JOIN Currencies \
            ON Configurations.currency = Currencies.name \
        WHERE Configurations.family_uid = ",
        parameter!(1),
    );

    /// Creates a new family configuration.
    ///
    /// # Arguments
    /// *  `currency` - The currency to use.
    #[cfg(test)]
    pub fn new(currency: Currency) -> Self {
        Self { currency }
    }

    /// Loads an item of this kind from the database.
    ///
    /// If no item corresponding to the keys exists, `Ok(None)` is
    /// returned.
    ///
    /// # Arguments
    /// *  `e` - The database executor.
    /// *  `family_uid` - The unique ID of the family whose configuration to
    ///    retrieve.
    pub async fn read<'a>(
        tx: &mut Tx<'a>,
        family_uid: &UID,
    ) -> Result<Option<Self>, DatabaseError> {
        if let Some(row) = sqlx::query(Self::READ)
            .bind(family_uid)
            .fetch_optional(tx.as_mut())
            .await?
        {
            let currency = Currency::from_row(&row)?;
            Ok(Some(Self { currency }))
        } else {
            Ok(None)
        }
    }

    /// The currency used by this family.
    pub fn currency(&self) -> &Currency {
        &self.currency
    }
}
