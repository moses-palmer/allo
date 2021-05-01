use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub use sqlx::sqlite::SqliteConnectOptions as ConnectOptions;

/// The database type.
pub type Database = sqlx::Sqlite;

/// The database configuration, in the form of a connection string.
#[derive(Clone, Deserialize, Serialize)]
pub struct Configuration {
    /// The connection string to use.
    connection_string: String,
}

/// The character used to represent parameters in SQL expressions.
macro_rules! parameter {
    ($name:ident) => {
        "?"
    };
}

/// The last inserted row ID.
macro_rules! last_row_id {
    () => {
        "SELECT last_insert_rowid()"
    };
}

/// Constructs a memory database for use with tests.
///
/// # Panics
/// This function will panic if the memory database pool cannot be created.
#[cfg(test)]
pub async fn test_pool() -> super::Pool {
    let pool = super::Pool::connect_with(
        Configuration {
            connection_string: "sqlite::memory:".into(),
        }
        .connect_options()
        .unwrap(),
    )
    .await
    .unwrap();

    pool
}

impl Configuration {
    /// Generates database connect options.
    pub fn connect_options(&self) -> Result<ConnectOptions, super::Error> {
        Ok(ConnectOptions::from_str(&self.connection_string)?
            .create_if_missing(true))
    }
}
