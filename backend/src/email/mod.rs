use lettre::message::Mailbox;
use lettre::Address;

use crate::db::values::EmailAddress;

mod sender;
pub mod template;

pub use self::sender::{Error, Sender};
pub use self::template::Templates;

#[cfg(not(feature = "email_smtp"))]
pub mod dummy;
#[cfg(not(feature = "email_smtp"))]
pub use dummy as driver;

#[cfg(feature = "email_smtp")]
pub mod smtp;
#[cfg(feature = "email_smtp")]
pub use smtp as driver;

pub use self::driver::Configuration;
pub use self::driver::Transport;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mailbox_valid() {
        assert_eq!(
            "Test <test@example.com>".parse::<Mailbox>().ok(),
            mailbox("Test", &"test@example.com".parse().unwrap()),
        );
    }
}
