mod currency_format;
pub use self::currency_format::*;
mod email_address;
pub use self::email_address::*;
mod password_hash;
pub use self::password_hash::*;
mod role;
pub use self::role::*;
mod schedule;
pub use self::schedule::*;
mod timestamp;
pub use self::timestamp::*;
mod transaction_type;
pub use self::transaction_type::*;
mod uid;
pub use self::uid::*;
mod url;
pub use self::url::*;

macro_rules! value {
    ($type:ty[$inner:ty]) => {
        impl<'r> Decode<'r, Database> for $type
        where
            &'r str: Decode<'r, Database>,
        {
            fn decode(
                value: <Database as HasValueRef<'r>>::ValueRef,
            ) -> Result<$type, Box<dyn Error + 'static + Send + Sync>> {
                Ok(Self(<$inner as Decode<'r, Database>>::decode(value)?))
            }
        }

        impl<'r> Encode<'r, Database> for $type
        where
            &'r str: Encode<'r, Database>,
        {
            fn encode_by_ref(
                &self,
                buf: &mut <Database as HasArguments<'r>>::ArgumentBuffer,
            ) -> IsNull {
                <$inner as Encode<'r, Database>>::encode_by_ref(&self.0, buf)
            }
        }

        impl Type<Database> for $type {
            fn type_info(
            ) -> <Database as weru::database::sqlx::Database>::TypeInfo {
                <$inner as Type<Database>>::type_info()
            }

            fn compatible(
                ty: &<Database as weru::database::sqlx::Database>::TypeInfo,
            ) -> bool {
                <$inner as Type<Database>>::compatible(ty)
            }
        }
    };
    ($type:ty => $inner:ty) => {
        impl<'r> Decode<'r, Database> for $type
        where
            &'r str: Decode<'r, Database>,
        {
            fn decode(
                value: <Database as HasValueRef<'r>>::ValueRef,
            ) -> Result<$type, Box<dyn Error + 'static + Send + Sync>> {
                let string = <String as Decode<'r, Database>>::decode(value)?;
                Ok(string.parse()?)
            }
        }

        impl<'r> Encode<'r, Database> for $type
        where
            &'r str: Encode<'r, Database>,
        {
            fn encode_by_ref(
                &self,
                buf: &mut <Database as HasArguments<'r>>::ArgumentBuffer,
            ) -> IsNull {
                let string = self.to_string();
                <String as Encode<'r, Database>>::encode_by_ref(&string, buf)
            }
        }

        impl Type<Database> for $type {
            fn type_info(
            ) -> <Database as weru::database::sqlx::Database>::TypeInfo {
                <$inner as Type<Database>>::type_info()
            }
        }
    };
}

mod values {
    use std::error::Error;

    use crate::db::values::*;
    use weru::database::sqlx::database::{HasArguments, HasValueRef};
    use weru::database::sqlx::encode::IsNull;
    use weru::database::sqlx::{Decode, Encode, Type};
    use weru::database::Database;

    value!(CurrencyFormat => String);
    value!(EmailAddress => String);
    value!(PasswordHash => String);
    value!(Role => String);
    value!(Schedule => String);
    value!(Timestamp[chrono::DateTime::<chrono::FixedOffset>]);
    value!(TransactionType => String);
    value!(UID => String);
    value!(URL => String);
}
