use std::convert::{TryFrom, TryInto};
use std::fs;
use std::io;
use std::path::PathBuf;

use lettre::message::Mailbox;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use toml;

use crate::db;
use crate::db::entities::Currency;
use crate::db::values::UID;
pub use crate::db::Configuration as Database;
use crate::email;
use crate::email::template::Language;
pub use crate::email::Configuration as EMailTransport;
use crate::notifications;
pub use crate::notifications::Configuration as Notifier;

use actix_session::CookieSession as SessionStorage;

/// The size, in bytes, of a key used to protect sessions.
const SESSION_KEY_SIZE: usize = 32;

#[derive(Clone, Deserialize, Serialize)]
pub struct Configuration {
    /// Server related configurations.
    pub server: Server,

    /// Session related configurations.
    pub session: Session,

    /// Database connection information.
    pub database: Database,

    /// The configuration for the notifier.
    pub notifier: Notifier,

    /// The configuration for the email sender.
    pub email: EMail,

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
pub struct Session {
    /// The secret used to protect cookies.
    pub secret: Secret<SESSION_KEY_SIZE>,

    /// Whether the cookie should be secure.
    pub secure: bool,

    /// The name of the cookie
    pub name: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct EMail {
    /// The file containing template definitions.
    pub templates: PathBuf,

    /// The default language to use when sending emails.
    pub default_language: Language,

    /// The name of the sender.
    pub from: Mailbox,

    /// The transport configuration.
    pub transport: EMailTransport,
}

/// A key used internally to maintain secrets.
///
/// When represented by a string, this is a string of length `SIZE * 2` of
/// hexadecimal characters. A byte is represented with the least significant
/// bits written first, so `0x12u8` will be read and written as `"21"`.
#[derive(Clone, Deserialize, Serialize)]
#[serde(into = "String")]
#[serde(try_from = "String")]
pub struct Secret<const SIZE: usize> {
    /// The key.
    pub key: [u8; SIZE],
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

    /// Connects to the database connection pool.
    pub async fn connection_pool(&self) -> Result<db::Pool, db::Error> {
        db::Pool::connect_with(self.database.connect_options()?).await
    }

    /// Connects to the notifier.
    pub async fn notifier<T>(
        &self,
    ) -> Result<impl notifications::Notifications, notifications::Error>
    where
        for<'a> T: Deserialize<'a>,
        T: Clone + Send + Sync + Serialize + 'static,
    {
        notifications::Notifier::<T>::new(&self.notifier).await
    }

    /// An email sender.
    ///
    /// This method will load the templates indicated in the configuration and
    /// generate an email transport.
    pub fn email_sender(
        &self,
    ) -> Result<
        email::Sender<impl lettre::AsyncTransport>,
        Box<dyn ::std::error::Error + Send + Sync>,
    > {
        Ok(email::Sender::new(
            email::Templates::load(&self.email.templates)?,
            self.email.from.clone(),
            self.email.transport.transport()?,
        ))
    }

    /// A session generator.
    pub fn session(&self) -> SessionStorage {
        self.session.storage()
    }

    /// The default configuration.
    pub fn defaults(&self) -> FamilyConfiguration {
        self.defaults.clone()
    }
}

impl<const SIZE: usize> Secret<SIZE> {
    /// The digits used when serialising and deserialising.
    const DIGITS: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D',
        'E', 'F',
    ];
}

impl<const SIZE: usize> TryFrom<String> for Secret<SIZE> {
    type Error = String;

    /// Attempts to construct a secret from a string.
    ///
    /// if `source` is an invalid string, an error is returned.
    ///
    /// # Arguments
    /// *  `source` - The source string.
    fn try_from(source: String) -> Result<Self, Self::Error> {
        Ok(Self {
            key: source
                .chars()
                .map(|c| c.to_digit(16))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    format!("secret <{}> contains an invalid character", source)
                })?
                .chunks(2)
                .map(|c| (c[0] | (c[1] << 4)) as u8)
                .collect::<Vec<_>>()
                .try_into()
                .map_err(|_| {
                    format!(
                        "expected length of secret <{}> to be {}, found {}",
                        source,
                        2 * SIZE,
                        source.len(),
                    )
                })?,
        })
    }
}

impl<const SIZE: usize> Into<String> for Secret<SIZE> {
    /// Converts this secret into a hexadecimal encoded string.
    fn into(self) -> String {
        (0..SIZE)
            .map(|i| {
                let byte = self.key[i >> 1];
                if i & 1 == 0 {
                    Self::DIGITS[(byte & 0x0F) as usize]
                } else {
                    Self::DIGITS[((byte >> 4) & 0x0F) as usize]
                }
            })
            .collect()
    }
}

impl Session {
    /// Generates the storage for this kind of session.
    pub fn storage(&self) -> SessionStorage {
        SessionStorage::signed(&self.secret.key)
            .secure(self.secure)
            .name(&self.name)
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
        parameter!(family_uid),
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
    pub async fn read<'a, E>(
        e: E,
        family_uid: &UID,
    ) -> Result<Option<Self>, db::Error>
    where
        E: ::sqlx::Executor<'a, Database = crate::db::Database>,
    {
        if let Some(row) = sqlx::query(Self::READ)
            .bind(family_uid)
            .fetch_optional(e)
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
