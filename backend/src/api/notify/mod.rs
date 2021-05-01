use std::sync::Arc;

use actix::{Actor, AsyncContext, StreamHandler};
use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use futures::stream::BoxStream;

use crate::api;
use crate::api::session::State;
use crate::db::values::UID;
use crate::notifications::{Error, Notifications, Notifier};

mod events;
pub use self::events::Event;

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
    notifier: web::Data<Arc<Notifier<Event>>>,
    session: Session,
    req: HttpRequest,
    payload: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    Ok(ws::start(
        NotificationSocket::new(
            stream(&notifier, &State::load(&session)?.user_uid).await?,
        ),
        &req,
        payload,
    )?)
}

/// Extracts a listening stream from a notifier for a given user.
///
/// # Arguments
/// *  `notifier` - The notifier.
/// *  `uid` - The user UID.
async fn stream(
    notifier: &Notifier<Event>,
    uid: &UID,
) -> Result<BoxStream<'static, Result<Event, Error>>, api::Error> {
    Ok(notifier.listen(&uid.to_string()).await?)
}

#[cfg(test)]
mod tests {
    use actix::prelude::*;

    use std::sync::mpsc::channel;

    use actix_web::body::to_bytes;
    use actix_web::test::TestRequest;
    use futures::stream;

    use crate::notifications::dummy::Notifier;

    use super::*;

    #[test]
    fn propagating() {
        let (tx, rx) = channel();

        let sys = System::new();
        sys.block_on(async move {
            let notifier = Notifier::new_with_events(vec![
                Event::Ping {},
                Event::Ping {},
                Event::Ping {},
            ]);
            let socket =
                NotificationSocket::new(notifier.listen("").await.unwrap());
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
