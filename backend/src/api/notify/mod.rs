use crate::prelude::*;

use weru::actix::session::Session;
use weru::actix::web::{web, HttpRequest, HttpResponse};
use weru::actix::web_actors::ws;
use weru::actix::{Actor, AsyncContext, StreamHandler};
use weru::channel::{Engine as ChannelEngine, Error};
use weru::futures::stream::BoxStream;
use weru::log;

use crate::api;
use crate::api::session::State;
use crate::db::entities::User;
use crate::db::values::{Role, UID};

mod events;
pub use self::events::Event;

/// A notification targeted at family members.
pub enum Notify {
    /// Sends a notification to a single user.
    Member { event: Event, user: UID },

    /// Sends a notification to all family members.
    Family { event: Event, family: UID },

    /// Notifies a single family member and their parents.
    MemberAndParents { event: Event, uid: UID, family: UID },

    /// Sends a notification to all parents.
    Parents { event: Event, family: UID },
}

impl Notify {
    /// Sends a notification to all affected users.
    ///
    /// If any error occurs during the operation, they are logged, but the
    /// error is then ignored.
    ///
    /// # Arguments
    /// *  `tx` - A database transaction.
    /// *  `channel` - The channel engine.
    /// *  `from` - The user that initiated the event. This user will not be
    ///    notified.
    pub async fn send<'a>(
        self,
        tx: &mut Tx<'a>,
        channel: &ChannelEngine,
        from: &UID,
    ) {
        let event = self.event();
        for uid in self.users(tx, from).await {
            let topic = uid.to_string();
            let channel = match channel.channel(&topic).await {
                Ok(channel) => channel,
                Err(e) => {
                    log::warn!(
                        "failed to acquire notification channel for topic {}: \
                        {}",
                        topic,
                        e,
                    );
                    return;
                }
            };
            if let Err(e) = channel.broadcast(event.clone()).await {
                log::warn!("failed to send notification to {}: {}", topic, e);
            }
        }
    }

    /// Extracts the event to send.
    fn event(&self) -> &Event {
        use Notify::*;
        match self {
            Member { event, .. }
            | Family { event, .. }
            | MemberAndParents { event, .. }
            | Parents { event, .. } => event,
        }
    }

    /// Lists all users that should be notified by this event.
    ///
    /// If any error occurs during the operation, they are logged, but the
    /// error is then ignored. In this case, an empty list is returned.
    ///
    /// # Arguments
    /// *  `tx` - A database transaction.
    /// *  `from` - The user that initiated the event. This user will not be
    ///    included.
    async fn users<'a>(&self, tx: &mut Tx<'a>, from: &UID) -> Vec<UID> {
        use Notify::*;
        match self {
            Member { user, .. } => vec![user.clone()],
            Family { family, .. } => {
                self.members(tx, family, |u| &u.uid != from).await
            }
            MemberAndParents { uid, family, .. } => {
                self.members(tx, family, |u| {
                    (&u.uid == uid || u.role == Role::Parent) && &u.uid != from
                })
                .await
            }
            Parents { family, .. } => {
                self.members(tx, family, |u| {
                    u.role == Role::Parent && &u.uid != from
                })
                .await
            }
        }
    }

    /// Loads all members of a family.
    ///
    /// If any error occurs during the operation, they are logged, but the
    /// error is then ignored. In this case, an empty list is returned.
    ///
    ///# Arguments
    /// *  `tx` - The database transaction.
    /// *  `family_uid` - The unique ID of the family.
    /// *  `predicae` - A filtering predicate.
    async fn members<'a, P>(
        &self,
        tx: &mut Tx<'a>,
        family_uid: &UID,
        predicate: P,
    ) -> Vec<UID>
    where
        P: Fn(&User) -> bool,
    {
        User::read_for_family(tx, family_uid)
            .await
            .unwrap_or_else(|e| {
                log::warn!("failed to load family: {}", e);
                vec![]
            })
            .into_iter()
            .filter_map(|u| {
                if predicate(&u) {
                    Some(u.uid.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Data for the notification web socket.
struct NotificationSocket {
    /// The stream providing notifications.
    ///
    /// Once the socket has started, this is cleared.
    stream: Option<BoxStream<'static, Result<Event, Error>>>,
}

impl NotificationSocket {
    /// Creates a new notification socket with a source of events from a
    /// stream.
    ///
    /// # Arguments
    /// *  `stream` - The event source.
    pub fn new(stream: BoxStream<'static, Result<Event, Error>>) -> Self {
        Self {
            stream: Some(stream),
        }
    }
}

impl Actor for NotificationSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        if let Some(stream) = self.stream.take() {
            ctx.add_stream(stream);
        }
    }
}

impl StreamHandler<Result<Event, Error>> for NotificationSocket {
    fn handle(&mut self, msg: Result<Event, Error>, ctx: &mut Self::Context) {
        match msg {
            Ok(Event::Logout {}) => ctx.close(None),
            Ok(event) => match serde_json::ser::to_string(&event) {
                Ok(json) => ctx.text(json),
                Err(e) => {
                    log::warn!("failed to deserialise notification: {}", e)
                }
            },
            Err(e) => log::warn!("failed to receive notification: {}", e),
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>>
    for NotificationSocket
{
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => ctx.close(reason),
            _ => (),
        }
    }
}

pub async fn handle(
    channel: web::Data<ChannelEngine>,
    session: Session,
    req: HttpRequest,
    payload: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    Ok(ws::start(
        NotificationSocket::new(
            stream(&channel, &State::load(&session)?.user_uid).await?,
        ),
        &req,
        payload,
    )?)
}

/// Extracts a listening stream from a notifier for a given user.
///
/// # Arguments
/// *  `channel` - The channel engine.
/// *  `uid` - The user UID.
async fn stream(
    channel: &ChannelEngine,
    uid: &UID,
) -> Result<BoxStream<'static, Result<Event, Error>>, api::Error> {
    Ok(channel.channel(&uid.to_string()).await?.listen().await?)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use weru::actix::web::body::to_bytes;
    use weru::actix::web::test::TestRequest;
    use weru::channel::engine::backends::local::Configuration;
    use weru::futures::stream;

    use super::*;

    #[test]
    fn propagating() {
        let (tx, rx) = mpsc::channel();

        let sys = System::new();
        sys.block_on(async move {
            let engine =
                ChannelConfiguration::Local(Configuration { queue_size: 16 })
                    .engine()
                    .await
                    .unwrap();
            let channel = engine.channel("test").await.unwrap();
            let socket =
                NotificationSocket::new(channel.listen().await.unwrap());
            let resp = ws::start(
                socket,
                &TestRequest::get()
                    .append_header(("connection", "upgrade"))
                    .append_header(("sec-websocket-key", "1123456"))
                    .append_header(("sec-websocket-version", "13"))
                    .append_header(("upgrade", "websocket"))
                    .to_http_request(),
                stream::pending(),
            )
            .unwrap();

            channel.broadcast(Event::Ping {}).await.unwrap();
            channel.broadcast(Event::Ping {}).await.unwrap();
            channel.broadcast(Event::Ping {}).await.unwrap();

            if let Some(body) = to_bytes(resp.into_body()).await.ok() {
                tx.send(Some(body)).unwrap();
            } else {
                tx.send(None).unwrap();
            }
        });

        let expected = actix_web::web::Bytes::from(
            b"\
            \x81\x0f{\"type\":\"Ping\"}\
            \x81\x0f{\"type\":\"Ping\"}\
            \x81\x0f{\"type\":\"Ping\"}"
                .iter()
                .map(|&b| b)
                .collect::<Vec<_>>(),
        );
        let actual = rx.recv().unwrap().unwrap();
        assert_eq!(actual, expected);
    }
}
