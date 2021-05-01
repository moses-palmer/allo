use actix::prelude::*;

use serde::{Deserialize, Serialize};

/// An event sent over the notification channel.
#[derive(Clone, Debug, Message, Deserialize, Serialize)]
#[rtype(result = "()")]
#[serde(tag = "type")]
pub enum Event {
    /// An empty ping message.
    Ping {},
}
