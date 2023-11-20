use crate::prelude::*;

use std::fmt;
use std::sync::PoisonError;

use serde::Serialize;
use weru::actix::web::error::BlockingError;
use weru::actix::web::http::StatusCode;
use weru::actix::web::{HttpResponse, Responder, ResponseError};
use weru::email::lettre::{message::Mailbox, Address};
use weru::log;

use crate::db::values::EmailAddress;

pub mod family;
pub mod invitation;
pub mod notify;
pub mod overview;
pub mod request;
pub mod server;
pub mod session;
pub mod transaction;
pub mod user;

/// A general grouping of errors returned by this API.
#[derive(Debug, PartialEq)]
pub enum Error {
    /// A context free error.
    Static(StatusCode, &'static str),

    /// A dynamic error.
    Dynamic(StatusCode, String),
}

impl Error {
    /// Generates an error indicating that the user is forbidden access to the
    /// requested resource.
    pub fn forbidden(reason: &'static str) -> Self {
        Self::Static(StatusCode::FORBIDDEN, reason)
    }

    /// Generates an error indicating that the user is unauthorized.
    pub fn unauthorized() -> Self {
        Self::Static(StatusCode::UNAUTHORIZED, "unauthorized")
    }

    /// Generates an error indicating the requested resource does not exist.
    pub fn not_found(reason: &'static str) -> Self {
        Self::Static(StatusCode::NOT_FOUND, reason)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match self {
            Static(_, s) => s.fmt(f),
            Dynamic(_, s) => s.fmt(f),
        }
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        use Error::*;
        match self {
            Static(c, _) | Dynamic(c, _) => *c,
        }
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(source: PoisonError<T>) -> Self {
        Error::Dynamic(StatusCode::INTERNAL_SERVER_ERROR, source.to_string())
    }
}

impl From<BlockingError> for Error {
    fn from(_: BlockingError) -> Self {
        Error::Static(
            StatusCode::INTERNAL_SERVER_ERROR,
            "waiting for connection pool cancelled",
        )
    }
}

macro_rules! errors_for_driver {
    ($code_var:ident for $driver:expr => {
        $($code:expr => $status:ident: $message:expr)+
    }) => {
        #[cfg(feature = $driver)]
        match $code_var.as_ref() {
            $(
                $code => Error::Static(StatusCode::$status, $message),
            )+
            e => {
                log::error!("An unexpected database error occurred: {}", e);
                Error::Static(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database error",
                )
            }
        }
    }
}

impl From<ChannelError> for Error {
    fn from(source: ChannelError) -> Self {
        log::error!("An unexpected notification error occurred: {}", source);
        Error::Static(StatusCode::NOT_FOUND, "no notifications")
    }
}

impl From<DatabaseError> for Error {
    fn from(source: DatabaseError) -> Self {
        match source {
            DatabaseError::Database(e) => {
                if let Some(code) = e.code() {
                    errors_for_driver!(code for "database-sqlite" => {
                        "2067" => CONFLICT: "entity exists"
                    })
                } else {
                    Error::Static(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "database error",
                    )
                }
            }
            e => {
                log::error!("An unexpected database error occurred: {}", e);
                Error::Static(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database error",
                )
            }
        }
    }
}

impl From<EMailError> for Error {
    fn from(source: EMailError) -> Self {
        log::error!("An unexpected email error occurred: {}", source);
        Error::Static(StatusCode::INTERNAL_SERVER_ERROR, "fail to send email")
    }
}

/// Generates an OK result.
pub fn ok<T>(result: T) -> Result<impl Responder, Error>
where
    T: Serialize,
{
    Ok(HttpResponse::Ok().json(result))
}

/// Converts an option to a result.
///
/// Use this to extract arguments from incoming data.
///
/// # Arguments
/// *  `a` - The argument to wrap.
pub fn argument<T>(a: Option<T>) -> Result<T, Error> {
    a.ok_or_else(|| Error::Static(StatusCode::BAD_REQUEST, "bad request"))
}

/// Converts an option to a result.
///
/// Use this to convert optionals generated internally.
///
/// # Arguments
/// *  `a` - The argument to wrap.
pub fn expect<T>(a: Option<T>) -> Result<T, Error> {
    a.ok_or_else(|| Error::Static(StatusCode::NOT_FOUND, "not found"))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::time::Duration;

    use weru::database::Connection;

    use crate::db::entities::create;
    use crate::db::entities::{Family, Request, Transaction, User};
    use crate::db::values::{Role, Timestamp, TransactionType};

    /// Populates the database with a default family with one parent, two
    /// children and transactions and requests.
    ///
    /// # Arguments
    /// *  `conn` - The database connection.
    pub fn populate(
        conn: &mut Connection,
    ) -> Result<
        (Family, User, (User, User), Vec<Transaction>, Vec<Request>),
        DatabaseError,
    > {
        let family = create::family(conn, "Family Name");
        let parent = create::user(
            conn,
            Role::Parent,
            "User Name",
            "test@email.com",
            &family.uid,
        );
        let children = (
            create::user(
                conn,
                Role::Child,
                "Child 1",
                "child1@example.com",
                &family.uid,
            ),
            create::user(
                conn,
                Role::Child,
                "Child 2",
                "child2@example.com",
                &family.uid,
            ),
        );
        let start = Timestamp::now();
        let transactions = (0..40)
            .map(|i| {
                create::transaction(
                    conn,
                    TransactionType::Gift,
                    if i & 1 != 0 {
                        &children.0.uid
                    } else {
                        &children.1.uid
                    },
                    &format!("description{}", i),
                    (i + 1) * 3,
                    start
                        .0
                        .checked_add_signed(
                            chrono::Duration::from_std(Duration::from_secs(
                                i as u64,
                            ))
                            .unwrap(),
                        )
                        .unwrap()
                        .into(),
                )
            })
            .collect::<Vec<_>>();
        let requests = (0..10)
            .map(|i| {
                create::request(
                    conn,
                    if i & 1 != 0 {
                        &children.0.uid
                    } else {
                        &children.1.uid
                    },
                    &format!("name{}", i),
                    &format!("description{}", i),
                    (i + 1) * 3,
                    "https://example.com/",
                )
            })
            .collect::<Vec<_>>();

        Ok((family, parent, children, transactions, requests))
    }
}

/// Creates a mailbox for a named user with an email address.
///
/// # Arguments
/// *  `name` - The user name.
/// *  `email` - The user email address. If this cannot be converted to a
///    mailbox address, nothing is returned.
pub fn mailbox(name: &str, email: &EmailAddress) -> Option<Mailbox> {
    email
        .to_string()
        .parse::<Address>()
        .ok()
        .map(|address| Mailbox::new(Some(name.into()), address))
}
