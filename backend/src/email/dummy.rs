use std::fmt;

use lettre::transport::stub::StubTransport;
use serde::{Deserialize, Serialize};

/// The dummy backend configuration.
#[derive(Clone, Deserialize, Serialize)]
pub struct Configuration {}

/// The errors that may occur during loading.
#[derive(Debug)]
pub enum Error {}

/// The transport type used by this driver.
pub type Transport = StubTransport;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        "no email relay configured".fmt(f)
    }
}

impl ::std::error::Error for Error {}

impl Configuration {
    /// Constructs a new transport from this configuration.
    ///
    /// A dummy email transport will always return an error.
    pub fn transport(&self) -> Result<Transport, Error> {
        Ok(StubTransport::new_error())
    }
}
