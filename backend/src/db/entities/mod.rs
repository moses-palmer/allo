use async_trait::async_trait;

/// A database entity.
#[async_trait]
pub trait Entity:
    for<'a> sqlx::FromRow<'a, <super::Database as sqlx::Database>::Row> + Unpin
{
    /// The type of the key value.
    type Key: Sync
        + for<'a> sqlx::Encode<'a, super::Database>
        + sqlx::Type<super::Database>;

    /// A description of an entity.
    ///
    /// This is not necessarily complete, and it does not contain a key.
    type Description;

    /// The SQL statement used to insert an item of this kind.
    const CREATE: &'static str;

    /// The SQL statement used to read a single item of this kind.
    const READ: &'static str;

    /// The SQL statement used to update an item of this kind.
    const UPDATE: &'static str;

    /// The SQL statement used to delete an item of this kind.
    const DELETE: &'static str;

    /// Inserts this item to the database.
    ///
    /// # Arguments
    /// *  `e` - The database executor.
    async fn create<'a, E>(&self, e: E) -> Result<(), super::Error>
    where
        E: ::sqlx::Executor<'a, Database = super::Database>;

    /// Loads an item of this kind from the database.
    ///
    /// If no item corresponding to the keys exists, `Ok(None)` is
    /// returned.
    ///
    /// # Arguments
    /// *  `e` - The database executor.
    async fn read<'a, E>(
        e: E,
        key: &Self::Key,
    ) -> Result<Option<Self>, super::Error>
    where
        E: ::sqlx::Executor<'a, Database = super::Database>,
    {
        sqlx::query_as(Self::READ).bind(key).fetch_optional(e).await
    }

    /// Updates this item in the database.
    ///
    /// # Arguments
    /// *  `e` - The database executor.
    async fn update<'a, E>(&self, e: E) -> Result<(), super::Error>
    where
        E: ::sqlx::Executor<'a, Database = super::Database>;

    /// Deletes this item from the database.
    ///
    /// # Arguments
    /// *  `e` - The database executor.
    async fn delete<'a, E>(&self, e: E) -> Result<(), super::Error>
    where
        E: sqlx::Executor<'a, Database = super::Database>,
    {
        let count = ::sqlx::query(Self::DELETE)
            .bind(self.key())
            .execute(e)
            .await?
            .rows_affected();
        if count > 0 {
            Ok(())
        } else {
            Err(crate::db::Error::RowNotFound)
        }
    }

    /// Merges a description into this entity.
    ///
    /// All values present in the description are set on this item.
    ///
    /// # Arguments
    /// *  `description` - The description to merge.
    fn merge(self, description: Self::Description) -> Self;

    /// The key of this item.
    fn key(&self) -> &Self::Key;
}

macro_rules! entity {
    ($(#[$doc:meta])* pub struct $name:ident in $table:ident {
        $(#[$key_doc:meta])*
        $key_name:ident: $key_type:ty,
        $(
            $(#[$field_doc:meta])*
            $field_name:ident: $field_type:ty,
        )+
    }) => {
        $(#[$doc])*
        #[derive(
            Clone,
            Debug,
            PartialEq,
            ::serde::Deserialize,
            ::serde::Serialize,
            ::sqlx::FromRow,
        )]
        pub struct $name {
            $(#[$key_doc])*
            $key_name: $key_type,
            $(
                $(#[$field_doc])*
                $field_name: $field_type,
            )*
        }

        /// A description of an entity.
        ///
        /// This struct contains all fields of the entity except the key.
        #[derive(
            Clone,
            Debug,
            Default,
            PartialEq,
            ::serde::Deserialize,
            ::serde::Serialize,
        )]
        pub struct Description {
            $(
                $(#[$field_doc])*
                pub $field_name: Option<$field_type>,
            )*
        }

        #[allow(unused)]
        impl Description {
            /// Merges this description with another.
            ///
            /// All items set in `other` will be copied to a new item.
            ///
            /// # Arguments
            /// *  `other` - Another description.
            pub fn merge(self, other: Self) -> Self {
                Self {
                    $(
                        $field_name: other.$field_name.or(self.$field_name),
                    )*
                }
            }

            /// Attempts to convert this description to an entity.
            ///
            /// Unless all fields are set, this method will return `None`.
            ///
            /// # Arguments
            /// *  `key` - The key value to use.
            pub fn entity(self, key: $key_type) -> Option<$name> {
                if [$(
                    self.$field_name.is_some(),
                )*].iter().all(|&o| o) {
                    Some($name {
                        $key_name: key,
                        $(
                            $field_name: self.$field_name.unwrap(),
                        )*
                    })
                } else {
                    None
                }
            }
        }

        #[allow(unused)]
        impl $name {
            /// Creates a new item of this kind.
            pub fn new(
                $key_name: $key_type,
                $($field_name: $field_type,)*
            ) -> Self {
                Self {
                    $key_name,
                    $($field_name,)*
                }
            }

            /// Lists all entities of this kind in the database.
            ///
            /// # Arguments
            /// *  `e` - The database executor.
            #[cfg(test)]
            pub async fn list<'a, E>(e: E) -> Result<Vec<Self>, crate::db::Error>
                where
                E: ::sqlx::Executor<'a, Database = crate::db::Database>,
            {
                use sqlx::FromRow;
                Ok(sqlx::query(concat!("SELECT * from ", stringify!($table)))
                    .fetch_all(e)
                    .await?
                    .iter()
                    .map(Self::from_row)
                    .map(Result::unwrap)
                    .collect())
            }

            $(#[$key_doc])*
            #[inline]
            pub fn $key_name(&self) -> &$key_type {
                &self.$key_name
            }

            $(
                $(#[$field_doc])*
                #[inline]
                pub fn $field_name(&self) -> &$field_type {
                    &self.$field_name
                }
            )*
        }

        #[allow(unused)]
        #[::async_trait::async_trait]
        impl crate::db::entities::Entity for $name {
            type Key = $key_type;
            type Description = Description;

            const CREATE: &'static str = concat!(
                "INSERT INTO ", stringify!($table), " (",
                    concat!($(stringify!($field_name), ", "),+),
                    stringify!($key_name),
                ") ",
                "VALUES (",
                    concat!($(parameter!($field_name), ", "),+),
                    parameter!($key_name),
                ")",
            );
            const READ: &'static str = concat!(
                "SELECT ", concat!($(stringify!($field_name), ", "),+),
                    stringify!($key_name), " ",
                "FROM ", stringify!($table), " ",
                "WHERE ", stringify!($key_name), " = ", parameter!($key_name),
            );
            const UPDATE: &'static str = concat!(
                "UPDATE ", stringify!($table), " ",
                "SET ",
                    concat!($(stringify!($field_name),
                        " = ", parameter!($field_name), ", "),+),
                    stringify!($key_name), " = ", parameter!($key_name),
                " ",
                "WHERE ", stringify!($key_name), " = ", parameter!($key_name),
            );
            const DELETE: &'static str = concat!(
                "DELETE FROM ", stringify!($table), " ",
                "WHERE ", stringify!($key_name), " = ", parameter!($key_name),
            );

            /// Inserts this item to the database.
            ///
            /// # Arguments
            /// *  `e` - The database executor.
            async fn create<'a, E>(
                &self,
                e: E,
            ) -> Result<(), crate::db::Error>
            where
                E: ::sqlx::Executor<'a, Database = crate::db::Database>,
            {
                let count = ::sqlx::query(Self::CREATE)
                    $(.bind(<$field_type>::from(self.$field_name.clone())))+
                    .bind(<$key_type>::from(self.$key_name.clone()))
                    .execute(e)
                    .await?
                    .rows_affected();
                if count != 1 {
                    Err(crate::db::Error::RowNotFound)
                } else {
                    Ok(())
                }
            }

            /// Loads an item of this kind from the database.
            ///
            /// If no item corresponding to the keys exists, `Ok(None)` is
            /// returned.
            ///
            /// # Arguments
            /// *  `e` - The database executor.
            async fn read<'a, E>(
                e: E,
                key: &$key_type,
            ) -> Result<Option<Self>, crate::db::Error>
            where
                E: ::sqlx::Executor<'a, Database = crate::db::Database>,
            {
                ::sqlx::query_as(Self::READ)
                    .bind(key)
                    .fetch_optional(e)
                    .await
            }

            /// Updates this item in the database.
            ///
            /// # Arguments
            /// *  `e` - The database executor.
            async fn update<'a, E>(
                &self,
                e: E,
            ) -> Result<(), crate::db::Error>
            where
                E: ::sqlx::Executor<'a, Database = crate::db::Database>,
            {
                let count = ::sqlx::query(Self::UPDATE)
                    $(.bind(self.$field_name.clone()))+
                    .bind(self.$key_name.clone())
                    .bind(self.$key_name.clone())
                    .execute(e)
                    .await?
                    .rows_affected();
                if count != 1 {
                    Err(crate::db::Error::RowNotFound)
                } else {
                    Ok(())
                }
            }

            fn key(&self) -> &Self::Key {
                &self.$key_name
            }

            fn merge(mut self, description: Self::Description) -> Self {
                $(
                    if let Some($field_name) = description.$field_name {
                        self.$field_name = $field_name;
                    }
                )*
                self
            }
        }
    };
}

macro_rules! entity_tests {
    (
        $entity_type:ty[$id_type:ty = $default_id:expr] {
            entity: $entity:expr;
            modify: $modify:expr;
            prepare: |$prepare_c:ident, $prepare_e:ident| $prepare:tt;
        }
    ) => {
        #[cfg(test)]
        #[allow(unused)]
        pub mod tests {
            use actix_rt;

            use crate::db;
            use crate::db::entities::Entity;
            use crate::db::test_pool;

            use super::*;

            /// The entity being tested.
            type EntityType = $entity_type;

            /// The type of the identifier field.
            type ID = $id_type;

            /// A comparable wrapper for errors.
            #[derive(Debug)]
            struct Error(db::Error);

            impl PartialEq for Error {
                fn eq(&self, other: &Self) -> bool {
                    false
                }
            }

            #[actix_rt::test]
            async fn test() {
                let modify: fn(EntityType) -> EntityType = $modify;

                let pool = test_pool().await;
                {
                    let mut connection = pool.acquire().await.unwrap();
                    let a = entity();
                    let id = a.key().clone();
                    let b = modify(a.clone());

                    // Sanity check test configuration
                    assert_eq!(&id, b.key(), "Identifiers do not match");

                    // Non-existing
                    assert_eq!(
                        EntityType::read(&mut connection, &id)
                            .await
                            .map_err(Error),
                        Ok(None),
                        "Reading missing value yielded result"
                    );

                    // Store and load
                    prepare(&mut connection, &a).await.unwrap();
                    assert_eq!(
                        a.create(&mut connection).await.map_err(Error),
                        Ok(()),
                        "Failed to create entity"
                    );
                    assert_eq!(
                        Ok(Some(a.clone())),
                        EntityType::read(&mut connection, &id)
                            .await
                            .map_err(Error),
                        "Store-load did not yield equal entity",
                    );

                    // Mutate
                    assert_eq!(
                        b.update(&mut connection).await.map_err(Error),
                        Ok(()),
                        "Failed to update entity",
                    );
                    assert_eq!(
                        Ok(Some(b.clone())),
                        EntityType::read(&mut connection, &id)
                            .await
                            .map_err(Error),
                        "Update did not mutate",
                    );

                    // Delete
                    assert_eq!(
                        a.delete(&mut connection).await.map_err(Error),
                        Ok(()),
                        "Failed to delete entity",
                    );
                    assert_eq!(
                        EntityType::read(&mut connection, &id)
                            .await
                            .map_err(Error),
                        Ok(None),
                        "Delete did not remove",
                    );
                }
            }

            /// Generates a default entity.
            pub fn entity() -> EntityType {
                entity_with_id($default_id)
            }

            /// Generates a default entity with a specific ID.
            ///
            /// # Arguments
            /// *  `id` - The ID to use.
            pub fn entity_with_id<T>(id: T) -> EntityType
            where
                T: Into<ID>,
            {
                let inner: fn(ID) -> EntityType = $entity;
                inner(id.into())
            }

            /// Prepares the database for storing an entity.
            ///
            /// # Arguments
            /// *  `c` - A database connection.
            /// *  `e` - The entity for which to prepare.
            pub async fn prepare(
                c: &mut db::Connection,
                e: &EntityType,
            ) -> Result<(), db::Error> {
                let $prepare_c = c;
                let $prepare_e = e;
                $prepare
            }
        }
    };
}

pub mod allowance;
pub use self::allowance::Allowance;
pub mod configuration;
pub use self::configuration::Configuration;
pub mod currency;
pub use self::currency::Currency;
pub mod family;
pub use self::family::Family;
pub mod invitation;
pub use self::invitation::Invitation;
pub mod password;
pub use self::password::Password;
pub mod request;
pub use self::request::Request;
pub mod transaction;
pub use self::transaction::Transaction;
pub mod user;
pub use self::user::User;

#[cfg(test)]
pub mod create {
    use super::*;

    use std::sync::atomic::{AtomicI64, Ordering};

    use futures::executor::block_on;

    use crate::db::values::*;
    use crate::db::Connection;

    pub fn allowance(
        c: &mut Connection,
        user_uid: &UID,
        amount: u32,
        schedule: Schedule,
    ) -> Allowance {
        let result =
            Allowance::new(UID::new(), user_uid.clone(), amount, schedule);
        block_on(result.create(c)).unwrap();
        result
    }

    pub fn family(c: &mut Connection, name: &str) -> Family {
        let result = Family::new(UID::new(), name.into());
        block_on(result.create(c)).unwrap();
        result
    }

    pub fn invitation(
        c: &mut Connection,
        role: Role,
        name: &str,
        email: &str,
        family_uid: &UID,
    ) -> Invitation {
        let (allowance_amount, allowance_schedule) = if role == Role::Child {
            (Some(42), Some("mon".parse().unwrap()))
        } else {
            (None, None)
        };
        let result = Invitation::new(
            UID::new(),
            role,
            name.into(),
            email.parse().unwrap(),
            allowance_amount,
            allowance_schedule,
            Timestamp::now(),
            family_uid.clone(),
        );
        block_on(result.create(c)).unwrap();
        result
    }

    pub fn password(c: &mut Connection, s: &str, user_uid: &UID) -> Password {
        let result = Password::new(
            user_uid.clone(),
            PasswordHash::from_password(s).unwrap(),
        );
        block_on(result.create(c)).unwrap();
        result
    }

    pub fn request(
        c: &mut Connection,
        user_uid: &UID,
        name: &str,
        description: &str,
        amount: i64,
        url: &str,
    ) -> Request {
        static mut UID: AtomicI64 = AtomicI64::new(0);
        let uid = unsafe { UID.fetch_add(1, Ordering::AcqRel) };
        let result = Request::new(
            uid,
            user_uid.clone(),
            name.into(),
            description.into(),
            amount,
            Some(url.parse().unwrap()),
            Timestamp::now(),
        );
        block_on(result.create(c)).unwrap();
        result
    }

    pub fn transaction(
        c: &mut Connection,
        transaction_type: TransactionType,
        user_uid: &UID,
        description: &str,
        amount: i64,
        timestamp: Timestamp,
    ) -> Transaction {
        static mut UID: AtomicI64 = AtomicI64::new(0);
        let uid = unsafe { UID.fetch_add(1, Ordering::AcqRel) };
        let result = Transaction::new(
            uid,
            transaction_type,
            user_uid.clone(),
            description.into(),
            amount,
            timestamp,
        );
        block_on(result.create(c)).unwrap();
        result
    }

    pub fn user(
        c: &mut Connection,
        role: Role,
        name: &str,
        email: &str,
        family_uid: &UID,
    ) -> User {
        let result = User::new(
            UID::new(),
            role,
            name.into(),
            Some(email.parse().unwrap()),
            family_uid.clone(),
        );
        block_on(result.create(c)).unwrap();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    entity!(
        pub struct Test in Tests {
            uid: i64,

            a: String,
            b: String,
        }
    );

    #[test]
    fn description_merge_no_overlap() {
        let a = Description {
            a: Some("a".into()),
            ..Default::default()
        };
        let b = Description {
            b: Some("b".into()),
            ..Default::default()
        };
        assert_eq!(
            a.merge(b),
            Description {
                a: Some("a".into()),
                b: Some("b".into())
            }
        );
    }

    #[test]
    fn description_merge_overlap() {
        let a = Description {
            a: Some("a".into()),
            ..Default::default()
        };
        let b = Description {
            a: Some("b".into()),
            ..Default::default()
        };
        assert_eq!(
            a.merge(b),
            Description {
                a: Some("b".into()),
                b: None,
            }
        );
    }

    #[test]
    fn description_entity_none() {
        let a = Description {
            a: Some("a".into()),
            ..Default::default()
        };
        assert!(a.entity(1).is_none());
    }

    #[test]
    fn description_entity_some() {
        let a = Description {
            a: Some("a".into()),
            b: Some("b".into()),
        };
        assert_eq!(
            a.entity(1),
            Some(Test {
                uid: 1,
                a: "a".into(),
                b: "b".into(),
            }),
        );
    }

    #[test]
    fn entity_merge_no_overlap() {
        let a = Test {
            uid: 42,
            a: "a".into(),
            b: "".into(),
        };
        let b = Description {
            b: Some("b".into()),
            ..Default::default()
        };
        assert_eq!(
            a.merge(b),
            Test {
                uid: 42,
                a: "a".into(),
                b: "b".into()
            }
        );
    }

    #[test]
    fn entity_merge_overlap() {
        let a = Test {
            uid: 42,
            a: "a".into(),
            b: "b".into(),
        };
        let b = Description {
            a: Some("b".into()),
            ..Default::default()
        };
        assert_eq!(
            a.merge(b),
            Test {
                uid: 42,
                a: "b".into(),
                b: "b".into(),
            }
        );
    }
}
