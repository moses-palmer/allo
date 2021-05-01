use lettre::transport::smtp::authentication::Mechanism;
use lettre::transport::smtp::AsyncSmtpTransport;
use serde::{Deserialize, Serialize};

/// The SMTP backend configuration.
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "protocol")]
pub enum Configuration {
    /// A simple relay server connecting to localhost on port 25.
    Localhost,

    /// A relay server using SMTPS.
    SMTPS {
        /// The SMTP host.
        server: String,

        /// The port.
        port: u16,

        /// The connection user name.
        username: String,

        /// The connection password.
        password: String,

        /// The authentication mechanism to use.
        mechanism: Mechanism,
    },

    /// A relay server using StartTLS.
    StartTLS {
        /// The SMTP host.
        server: String,

        /// The port.
        port: u16,

        /// The connection user name.
        username: String,

        /// The connection password.
        password: String,

        /// The authentication mechanism to use.
        mechanism: Mechanism,
    },
}

/// The errors generated by the transport.
pub type Error = lettre::transport::smtp::Error;

/// The transport type used by this driver.
pub type Transport = AsyncSmtpTransport<lettre::Tokio1Executor>;

impl Configuration {
    /// Constructs a new transport from this configuration.
    pub fn transport(&self) -> Result<Transport, Error> {
        use Configuration::*;
        Ok(match self {
            Localhost => Transport::unencrypted_localhost(),
            SMTPS {
                server,
                port,
                username,
                password,
                mechanism,
            } => Transport::relay(server)?
                .port(*port)
                .credentials((username, password).into())
                .authentication(vec![mechanism.clone()])
                .build(),
            StartTLS {
                server,
                port,
                username,
                password,
                mechanism,
            } => Transport::starttls_relay(server)?
                .port(*port)
                .credentials((username, password).into())
                .authentication(vec![mechanism.clone()])
                .build(),
        })
    }
}