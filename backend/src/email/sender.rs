use lettre;

use lettre::message::{Attachment, Mailbox, Message, MultiPart, SinglePart};
use lettre::AsyncTransport;

use super::template::{Language, TemplateName, Templates};

/// An error occurring when sending an email.
#[derive(Debug)]
pub enum Error<T> {
    /// The template does not exist.
    Template,

    /// The content to send is invalid.
    Content(lettre::error::Error),

    /// An error occurred when attempting to send the email.
    Transport(T),
}

/// A sender of emails.
///
/// This struct maintains a selection of templates
pub struct Sender<T>
where
    T: AsyncTransport + Sync,
{
    /// The templates used by this email sender.
    templates: Templates,

    /// The mailbox indicated by the _From_ header.
    from: Mailbox,

    /// The actual email transport method.
    transport: T,
}

impl<T> ::std::fmt::Display for Error<T>
where
    T: ::std::error::Error,
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            Error::Template => write!(f, "unknown template"),
            Error::Content(e) => e.fmt(f),
            Error::Transport(e) => <T as ::std::fmt::Display>::fmt(e, f),
        }
    }
}

impl<T> From<lettre::error::Error> for Error<T> {
    fn from(source: lettre::error::Error) -> Self {
        Self::Content(source)
    }
}

#[cfg(feature = "email_smtp")]
impl From<lettre::transport::smtp::Error>
    for Error<lettre::transport::smtp::Error>
{
    fn from(source: lettre::transport::smtp::Error) -> Self {
        Self::Transport(source)
    }
}

impl From<lettre::transport::stub::Error>
    for Error<lettre::transport::stub::Error>
{
    fn from(source: lettre::transport::stub::Error) -> Self {
        Self::Transport(source)
    }
}

impl<T> ::std::error::Error for Error<T> where T: ::std::error::Error {}

impl<T> Sender<T>
where
    T: AsyncTransport + Sync,
    Error<T::Error>: From<T::Error>,
{
    /// Creates a new email sender.
    /// *  `templates` - The templates used by this email sender.
    /// *  `from` - The mailbox indicated by the _From_ header.
    /// *  `transport` - The actual email transport method.
    pub fn new(templates: Templates, from: Mailbox, transport: T) -> Self {
        Self {
            templates,
            from,
            transport,
        }
    }

    /// Sends an email to the recipient specified.
    ///
    /// # Arguments
    /// *  `recipient` - The single recipient of the message.
    /// *  `languages` - A sequence of langauges to use, in decreasing order of
    ///    relevance. The first langauge for which the template exists is used.
    /// *  `template` - The template name.
    /// *  `replacements` - A function converting keys to replacement strings.
    ///    If this function returns `None`, the replacement string is kept.
    pub async fn send<'a, F, I>(
        &self,
        recipient: Mailbox,
        languages: I,
        template: &TemplateName,
        replacements: F,
    ) -> Result<(), Error<T::Error>>
    where
        F: Fn(&str) -> Option<&'a str> + 'a,
        I: IntoIterator<Item = Language>,
    {
        let template = languages
            .into_iter()
            .find_map(|language| self.templates.get(&language, template))
            .ok_or(Error::Template)?;
        let message = Message::builder()
            .from(self.from.clone())
            .subject(template.subject())
            .to(recipient)
            .multipart(
                template.attachments().iter().fold(
                    MultiPart::related()
                        .singlepart(SinglePart::html(
                            template.html(|key| replacements(key)),
                        ))
                        .singlepart(SinglePart::plain(
                            template.text(|key| replacements(key)),
                        )),
                    |multipart, (name, attachment)| {
                        multipart.singlepart(
                            Attachment::new_inline(name.into()).body(
                                attachment.data().to_vec(),
                                attachment.content_type().clone(),
                            ),
                        )
                    },
                ),
            )?;

        Ok(self.transport.send(message).await.map(|_| ())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    use lettre::transport::stub::{Error as StubError, StubTransport};

    use crate::email::template::Templates;

    #[actix_rt::test]
    async fn fails_for_unknown() {
        let sender =
            Sender::new(templates(), mailbox(), StubTransport::new_ok());

        assert_eq!(
            Err(Error::<StubError>::Template.to_string()),
            sender
                .send(mailbox(), ["l1".into()], &"tx".into(), |_| None)
                .await
                .map_err(|e| e.to_string()),
        );
        assert_eq!(
            Err(Error::<StubError>::Template.to_string()),
            sender
                .send(mailbox(), ["lx".into()], &"t1".into(), |_| None)
                .await
                .map_err(|e| e.to_string()),
        );
    }

    #[actix_rt::test]
    async fn fails_for_transport_error() {
        let sender =
            Sender::new(templates(), mailbox(), StubTransport::new_error());

        assert_eq!(
            Err(Error::Transport(Box::new(StubError)).to_string()),
            sender
                .send(mailbox(), ["l1".into()], &"t1".into(), |_| None)
                .await
                .map_err(|e| e.to_string()),
        );
    }

    #[actix_rt::test]
    async fn succeeds_for_no_error() {
        let sender =
            Sender::new(templates(), mailbox(), StubTransport::new_ok());

        assert_eq!(
            Ok(()),
            sender
                .send(mailbox(), ["l1".into()], &"t1".into(), |_| None)
                .await
                .map_err(|e| e.to_string()),
        );
    }

    /// Loads the valid templates from the test resource directory.
    fn templates() -> Templates {
        Templates::load(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources/test/email/template/valid.toml"),
        )
        .unwrap()
    }

    /// A simple mailbox.
    fn mailbox() -> Mailbox {
        "Tester <test@test.com>".parse().unwrap()
    }
}
