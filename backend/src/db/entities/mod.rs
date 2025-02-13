#![allow(unused_imports)]

macro_rules! entity_tests {
    (
        $entity_type:ty[$id_type:ty = $default_id:expr] {
            entity: $entity:expr;
            modify: $modify:expr;
            prepare: |$prepare_tx:ident, $prepare_e:ident| $prepare:tt;
        }
    ) => {
        #[cfg(test)]
        #[allow(unused)]
        pub mod tests {
            use weru::database::{
                sqlx::Acquire, Entity, Error as DatabaseError,
                Transaction as Tx,
            };

            use crate::db::test_engine;

            use super::*;

            /// The entity being tested.
            type EntityType = $entity_type;

            /// The type of the identifier field.
            type ID = $id_type;

            /// A comparable wrapper for errors.
            #[derive(Debug)]
            struct Error(DatabaseError);

            impl PartialEq for Error {
                fn eq(&self, other: &Self) -> bool {
                    false
                }
            }

            #[actix_rt::test]
            async fn test() {
                let modify: fn(EntityType) -> EntityType = $modify;

                let database = test_engine().await;
                let mut conn = database.connection().await.expect("connection");
                let a = entity();
                let id = a.key().clone();
                let b = modify(a.clone());

                // Sanity check test configuration
                assert_eq!(&id, b.key(), "Identifiers do not match");

                // Non-existing
                {
                    let mut tx = conn.begin().await.expect("transaction");
                    assert_eq!(
                        EntityType::read(tx.as_mut(), &id).await.map_err(Error),
                        Ok(None),
                        "Reading missing value yielded result"
                    );
                    tx.commit().await.expect("commit");
                }

                // Store and load
                {
                    let mut tx = conn.begin().await.expect("transaction");
                    prepare(&mut tx, &a).await.expect("prepare");
                    assert_eq!(
                        a.create(tx.as_mut()).await.expect("create"),
                        (),
                        "Failed to create entity"
                    );
                    tx.commit().await.expect("commit");
                }
                {
                    let mut tx = conn.begin().await.expect("transaction");
                    assert_eq!(
                        EntityType::read(tx.as_mut(), &id).await.map_err(Error),
                        Ok(Some(a.clone())),
                        "Store-load did not yield equal entity",
                    );
                    tx.commit().await.expect("commit");
                }

                // Mutate
                {
                    let mut tx = conn.begin().await.expect("transaction");
                    assert_eq!(
                        b.update(tx.as_mut()).await.expect("update"),
                        (),
                        "Failed to update entity",
                    );
                    tx.commit().await.expect("commit");
                }
                {
                    let mut tx = conn.begin().await.expect("transaction");
                    assert_eq!(
                        EntityType::read(tx.as_mut(), &id).await.map_err(Error),
                        Ok(Some(b.clone())),
                        "Update did not mutate",
                    );
                    tx.commit().await.expect("commit");
                }

                // Delete
                {
                    let mut tx = conn.begin().await.expect("transaction");
                    assert_eq!(
                        a.delete(tx.as_mut()).await.map_err(Error),
                        Ok(()),
                        "Failed to delete entity",
                    );
                    tx.commit().await.expect("commit");
                }
                {
                    let mut tx = conn.begin().await.expect("transaction");
                    assert_eq!(
                        EntityType::read(tx.as_mut(), &id).await.map_err(Error),
                        Ok(None),
                        "Delete did not remove",
                    );
                    tx.commit().await.expect("commit");
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
            /// *  `tx` - A database transaction.
            /// *  `e` - The entity for which to prepare.
            pub async fn prepare<'a>(
                tx: &mut Tx<'a>,
                e: &EntityType,
            ) -> Result<(), DatabaseError> {
                let $prepare_tx = tx;
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

    use weru::database::{Connection, Entity};

    use weru::futures::executor::block_on;

    use crate::db::values::*;

    pub fn allowance(
        conn: &mut Connection,
        user_uid: &UID,
        amount: u32,
        schedule: Schedule,
    ) -> Allowance {
        let result =
            Allowance::new(UID::new(), user_uid.clone(), amount, schedule);
        block_on(result.create(conn.as_mut())).unwrap();
        result
    }

    pub fn family(conn: &mut Connection, name: &str) -> Family {
        let result = Family::new(UID::new(), name.into());
        block_on(result.create(conn.as_mut())).unwrap();
        result
    }

    pub fn invitation(
        conn: &mut Connection,
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
        block_on(result.create(conn.as_mut())).unwrap();
        result
    }

    pub fn password(
        conn: &mut Connection,
        s: &str,
        user_uid: &UID,
    ) -> Password {
        let result = Password::new(
            user_uid.clone(),
            PasswordHash::from_password(s).unwrap(),
        );
        block_on(result.create(conn.as_mut())).unwrap();
        result
    }

    #[allow(static_mut_refs)]
    pub fn request(
        conn: &mut Connection,
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
        block_on(result.create(conn.as_mut())).unwrap();
        result
    }

    #[allow(static_mut_refs)]
    pub fn transaction(
        conn: &mut Connection,
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
        block_on(result.create(conn.as_mut())).unwrap();
        result
    }

    pub fn user(
        conn: &mut Connection,
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
        block_on(result.create(conn.as_mut())).unwrap();
        result
    }
}

#[cfg(test)]
mod tests {
    use weru::database::{entity, Entity};

    #[derive(Debug, PartialEq)]
    #[entity(Tests)]
    pub struct Test {
        pub uid: i64,

        pub a: String,
        pub b: String,
    }

    #[test]
    fn description_merge_no_overlap() {
        let a = TestDescription {
            a: Some("a".into()),
            ..Default::default()
        };
        let b = TestDescription {
            b: Some("b".into()),
            ..Default::default()
        };
        assert_eq!(
            a.merge(b),
            TestDescription {
                a: Some("a".into()),
                b: Some("b".into())
            }
        );
    }

    #[test]
    fn description_merge_overlap() {
        let a = TestDescription {
            a: Some("a".into()),
            ..Default::default()
        };
        let b = TestDescription {
            a: Some("b".into()),
            ..Default::default()
        };
        assert_eq!(
            a.merge(b),
            TestDescription {
                a: Some("b".into()),
                b: None,
            }
        );
    }

    #[test]
    fn description_entity_none() {
        let a = TestDescription {
            a: Some("a".into()),
            ..Default::default()
        };
        assert!(a.entity(1).is_none());
    }

    #[test]
    fn description_entity_some() {
        let a = TestDescription {
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
        let b = TestDescription {
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
        let b = TestDescription {
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
