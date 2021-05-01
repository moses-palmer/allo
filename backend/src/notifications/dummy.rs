use async_trait::async_trait;
use futures::stream;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Configuration;

/// A dummy notifier that only yields predefined events.
pub struct Notifier<T> {
    /// Any pre-polated events.
    events: Vec<T>,

    _m: ::std::marker::PhantomData<T>,
}

#[allow(unused)]
impl<T> Notifier<T>
where
    for<'a> T: Deserialize<'a>,
    T: Send + Sync + Serialize,
{
    /// Creates a new Redis notification manager.
    ///
    /// # Arguments
    /// *  `configuration` - The connection configuration.
    pub async fn new(
        _configuration: &Configuration,
    ) -> Result<Self, super::Error> {
        Ok(Self {
            events: Vec::new(),
            _m: ::std::marker::PhantomData,
        })
    }

    /// Constructs a dummy event notifier with a collection of events.
    ///
    /// # Arguments
    /// *  `events` - The events to yield.
    pub fn new_with_events(events: Vec<T>) -> Self {
        Self {
            events,
            _m: ::std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T> super::Notifications for Notifier<T>
where
    for<'a> T: Deserialize<'a>,
    T: Clone + Send + Sync + Serialize + 'static,
{
    type Event = T;

    async fn send(
        &self,
        _channel: &str,
        _event: &Self::Event,
    ) -> Result<(), super::Error> {
        Err(super::Error::Driver("not implemented".into()))
    }

    async fn listen(
        &self,
        _channel: &str,
    ) -> Result<
        BoxStream<'static, Result<Self::Event, super::Error>>,
        super::Error,
    > {
        let events = self
            .events
            .iter()
            .map(|e| Ok(e.clone()))
            .collect::<Vec<_>>();
        Ok(Box::pin(stream::iter(events)))
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use serde::{Deserialize, Serialize};

    use crate::notifications::Notifications;

    use super::*;

    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    struct Event {
        message: String,
    }

    #[actix_rt::test]
    async fn listen() {
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
        let notifier = Notifier::new_with_events(expected.clone());

        let stream = notifier.listen(channel).await.unwrap();

        assert_eq!(
            stream
                .take(expected.len())
                .map(Result::unwrap)
                .collect::<Vec<_>>()
                .await,
            expected,
        );
    }
}
