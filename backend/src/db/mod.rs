#[cfg(feature = "db_sqlite")]
#[macro_use]
#[path = "sqlite.rs"]
mod driver;

pub mod entities;
pub mod values;

pub use self::driver::{Configuration, Database, MIGRATOR};

#[cfg(test)]
pub use self::driver::test_pool;

/// A connection pool.
pub type Pool = sqlx::Pool<Database>;

/// A pooled connection.
pub type Connection = sqlx::pool::PoolConnection<Database>;

/// A transaction.
pub type Transaction<'a> = sqlx::Transaction<'a, Database>;

/// A database error.
pub type Error = sqlx::Error;
