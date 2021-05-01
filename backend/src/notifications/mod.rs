use std::error;
use std::fmt;

use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

/// An error relating to notifications.
#[derive(Debug)]
pub enum Error {
    /// An error from the underlying driver.
    Driver(Box<dyn error::Error + Send + Sync>),

    /// An error from serialisation or deserialisation.
    Serialization(Box<dyn error::Error + Send + Sync>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match self {
            Driver(e) => e.fmt(f),
            Serialization(e) => e.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[async_trait]
pub trait Notifications {
    /// The type of events sent as notifications.
    type Event: Send + Serialize + for<'a> Deserialize<'a>;

    /// Sends a notification over a notification channel.
    ///
    /// # Arguments
    /// *  `channel` - The channel over which to send the notification event.
    /// *  `event` - The event to send.
    async fn send(
        &self,
        channel: &str,
        event: &Self::Event,
    ) -> Result<(), Error>;

    /// Begins listening on a channel.
    ///
    /// # Arguments
    /// *  `channel` - The channel over which to listen.
    async fn listen(
        &self,
        channel: &str,
    ) -> Result<BoxStream<'static, Result<Self::Event, Error>>, Error>;
}
