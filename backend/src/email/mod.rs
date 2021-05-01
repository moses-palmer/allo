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
