#[cfg(test)]
use weru::database::{Configuration, Engine};

/// A migrator for the database schema.
pub const MIGRATOR: weru::database::sqlx::migrate::Migrator =
    weru::database::sqlx::migrate!("src/db/migrations/sqlite/");

macro_rules! sql_from_file {
    ($name:expr) => {
        include_str!(concat!($name, ".sqlite.sql"))
    };
}

/// Constructs a memory database for use with tests.
///
/// # Panics
/// This function will panic if the memory database pool cannot be created.
#[cfg(test)]
pub async fn test_engine() -> Engine {
    let engine = Configuration {
        connection_string: "sqlite::memory:".into(),
    }
    .engine()
    .await
    .expect("test engine");

    MIGRATOR
        .run(&mut engine.connection().await.expect("database connection"))
        .await
        .expect("database migration");

    engine
}
