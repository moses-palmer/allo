use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Configuration {
    /// The connection information.
    connection_string: String,

    /// The prefix used to generate a channel name.
    channel_prefix: String,
}

pub struct Notifier<T> {
    /// The redis client.
    client: Client,

    /// The Redis connection used to write messages.
    conn: MultiplexedConnection,

    /// The prefix used to generate a channel name.
    channel_prefix: String,

    _m: ::std::marker::PhantomData<T>,
}

impl<T> Notifier<T>
where
    for<'a> T: Deserialize<'a>,
    T: Clone + Send + Sync + Serialize,
{
    /// Creates a new Redis notification manager.
    ///
    /// # Arguments
    /// *  `configuration` - The connection configuration.
    pub async fn new(
        configuration: &Configuration,
    ) -> Result<Self, super::Error> {
        let client = Client::open(configuration.connection_string.clone())?;
        let conn = client.get_multiplexed_async_connection().await?;
        Ok(Self {
            client,
            conn,
            channel_prefix: configuration.channel_prefix.clone(),
            _m: ::std::marker::PhantomData,
        })
    }

    /// Generates the fully qualified name of a channel.
    ///
    /// # Arguments
    /// *  `channel` - The simple channel name.
    fn channel(&self, channel: &str) -> String {
        format!("{}.{}", self.channel_prefix, channel)
    }
}

#[async_trait]
impl<T> super::Notifications for Notifier<T>
where
    for<'a> T: Deserialize<'a>,
    T: Clone + Send + Sync + Serialize,
{
    type Event = T;

    async fn send(
        &self,
        channel: &str,
        event: &Self::Event,
    ) -> Result<(), super::Error> {
        let bytes = serde_cbor::to_vec(event)?;
        Ok(self
            .conn
            .clone()
            .publish(self.channel(channel), bytes)
            .await?)
    }

    async fn listen(
        &self,
        channel: &str,
    ) -> Result<
        BoxStream<'static, Result<Self::Event, super::Error>>,
        super::Error,
    > {
        let mut pubsub =
            self.client.get_async_connection().await?.into_pubsub();
        pubsub.subscribe(self.channel(channel)).await?;
        Ok(Box::pin(pubsub.into_on_message().map(|msg| {
            let bytes = msg.get_payload_bytes();
            Ok(serde_cbor::from_slice(&bytes)?)
        })))
    }
}

impl From<redis::RedisError> for super::Error {
    fn from(source: redis::RedisError) -> Self {
        super::Error::Driver(Box::new(source))
    }
}

impl From<serde_cbor::Error> for super::Error {
    fn from(source: serde_cbor::Error) -> Self {
        super::Error::Serialization(Box::new(source))
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use serde::{Deserialize, Serialize};
    use serial_test::serial;

    use crate::notifications::Notifications;

    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    struct Event {
        message: String,
    }

    macro_rules! notifier {
        () => {
            if let Ok(connection_string) =
                ::std::env::var("ALLO_REDIS_CONNECTION_STRING")
            {
                let configuration = super::Configuration {
                    connection_string,
                    channel_prefix: "test".into(),
                };
                super::Notifier::<Event>::new(&configuration).await.unwrap()
            } else {
                println!(
                    "Ignoring test since $ALLO_REDIS_CONNECTION_STRING is not \
                    set",
                );
                return;
            }
        };
    }

    #[actix_rt::test]
    #[serial]
    async fn pubsub_single() {
        let notifier = notifier!();
        let channel = "test";
        let expected = vec![
            Event {
                message: "one".into(),
            },
            Event {
                message: "two".into(),
            },
            Event {
                message: "three".into(),
            },
        ];

        let stream = notifier.listen(channel).await.unwrap();
        for event in &expected {
            notifier.send(channel, event).await.unwrap();
        }

        assert_eq!(
            stream
                .take(expected.len())
                .map(Result::unwrap)
                .collect::<Vec<_>>()
                .await,
            expected,
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn pubsub_multi() {
        let notifier = notifier!();
        let channel = "test";
        let expected = vec![
            Event {
                message: "one".into(),
            },
            Event {
                message: "two".into(),
            },
            Event {
                message: "three".into(),
            },
        ];

        let stream1 = notifier.listen(channel).await.unwrap();
        let stream2 = notifier.listen(channel).await.unwrap();
        for event in &expected {
            notifier.send(channel, event).await.unwrap();
        }

        assert_eq!(
            stream1
                .take(expected.len())
                .map(Result::unwrap)
                .collect::<Vec<_>>()
                .await,
            expected,
        );
        assert_eq!(
            stream2
                .take(expected.len())
                .map(Result::unwrap)
                .collect::<Vec<_>>()
                .await,
            expected,
        );
    }
}
