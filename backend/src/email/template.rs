use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};

use lettre::message::header::ContentType;
use pulldown_cmark::html;
use pulldown_cmark::{Event, Parser, Tag};
use serde::{Deserialize, Serialize};

/// A language code.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Language(String);

/// A template name.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TemplateName(String);

/// An error occurring when loading templates.
#[derive(Debug)]
pub enum Error {
    /// An error occurred when parsing the input file.
    Parse(Box<dyn ::std::error::Error + Send + Sync>),

    /// An error occurred when reading a referenced file.
    Reference(
        Language,
        TemplateName,
        PathBuf,
        Box<dyn ::std::error::Error + Send + Sync>,
    ),
}

/// A collection of templates grouped into first language code and then name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Templates(HashMap<Language, HashMap<TemplateName, Template>>);

/// An attachment.
#[derive(Clone, Debug, PartialEq)]
pub struct Attachment {
    /// The content type of this attachment.
    content_type: ContentType,

    /// The actual file data.
    data: Vec<u8>,
}

/// An email template.
///
/// A template allows interpolating strings on the form `"${token}"` into a
/// markdown document, and then converting it to a different format.
///
/// Characters in a token cannot be escaped; a token is consumed in its
/// entirety until a closing bracket is encountered.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Template {
    /// The text wrapping the HTML.
    wrapping: (String, String),

    /// The message subject.
    subject: String,

    /// The markdown body.
    body: String,

    /// Additional named attachments.
    attachments: HashMap<String, Attachment>,
}

impl<T> From<T> for Language
where
    String: From<T>,
{
    fn from(source: T) -> Self {
        Self(source.into())
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> From<T> for TemplateName
where
    String: From<T>,
{
    fn from(source: T) -> Self {
        Self(source.into())
    }
}

impl fmt::Display for TemplateName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match self {
            Parse(e) => write!(f, "failed to parse input file: {}", e),
            Reference(l, n, p, e) => {
                let p = p.to_string_lossy();
                write!(f, "failed to read {} for {}/{}: {}", p, l, n, e,)
            }
        }
    }
}

impl ::std::error::Error for Error {}

/// A description of a single attachment.
#[derive(Deserialize, Serialize)]
struct AttachmentDescription {
    /// The content type of this file.
    content_type: String,

    /// The path, relative to the template file, of the data.
    path: String,
}

impl Eq for Attachment {}

impl Attachment {
    /// The content type of this attachment.
    pub fn content_type(&self) -> &ContentType {
        &self.content_type
    }

    /// The data of this attachment.
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

/// A description of a single template.
#[derive(Deserialize, Serialize)]
struct TemplateDescription {
    /// The file containing the wrapper.
    wrapping: String,

    /// The subject of the message.
    subject: String,

    /// The file containing the body.
    body: String,

    /// The files containing the attachments.
    attachments: HashMap<String, AttachmentDescription>,
}

impl TemplateDescription {
    /// Attempts to load the file specified as wrapping.
    ///
    /// # Arguments
    /// *  `language` - The language for this template. This is used to
    ///    generate an error message.
    /// *  `template_name` - The name of this template. This is used to
    ///    generate an error message.
    /// *  `parent` - The parent directory. This is used to generate the full
    ///    path name.
    pub fn wrapping_data<P>(
        &self,
        language: &Language,
        template_name: &TemplateName,
        parent: P,
    ) -> Result<String, Error>
    where
        P: AsRef<Path>,
    {
        Self::load_string(
            language,
            template_name,
            parent.as_ref().join(&self.wrapping),
        )
    }

    /// Attempts to load the file specified as body.
    ///
    /// # Arguments
    /// *  `language` - The language for this template. This is used to
    ///    generate an error message.
    /// *  `template_name` - The name of this template. This is used to
    ///    generate an error message.
    /// *  `parent` - The parent directory. This is used to generate the full
    ///    path name.
    pub fn body_data<P>(
        &self,
        language: &Language,
        name: &TemplateName,
        parent: P,
    ) -> Result<String, Error>
    where
        P: AsRef<Path>,
    {
        Self::load_string(language, name, parent.as_ref().join(&self.body))
    }

    /// Attempts to load the files specified as attachments.
    ///
    /// # Arguments
    /// *  `language` - The language for this template. This is used to
    ///    generate an error message.
    /// *  `template_name` - The name of this template. This is used to
    ///    generate an error message.
    /// *  `parent` - The parent directory. This is used to generate the full
    ///    path name.
    pub fn attachments_data<P>(
        &self,
        language: &Language,
        template_name: &TemplateName,
        parent: P,
    ) -> Result<HashMap<String, Attachment>, Error>
    where
        P: AsRef<Path>,
    {
        self.attachments
            .iter()
            .map(|(name, attachment)| {
                Ok((
                    name.clone(),
                    Attachment {
                        content_type: ContentType::parse(
                            &attachment.content_type,
                        )
                        .map_err(|e| Error::Parse(Box::new(e)))?,
                        data: Self::load(
                            language,
                            template_name,
                            parent.as_ref().join(&attachment.path),
                            Ok,
                        )?,
                    },
                ))
            })
            .collect::<Result<HashMap<_, _>, _>>()
    }

    /// Attempts to load a file.
    ///
    /// The binary data read is passed to a mapper function that provides the
    /// final result.
    ///
    /// # Arguments
    /// *  `language` - The language for this template. This is used to
    ///    generate an error message.
    /// *  `name` - The name of this template. This is used to generate and
    ///    error message.
    /// *  `path` - The full path to the file to read.
    /// *  `mapper`- A function converting the binary data to a result.
    fn load<F, T>(
        language: &Language,
        name: &TemplateName,
        path: PathBuf,
        mapper: F,
    ) -> Result<T, Error>
    where
        F: Fn(Vec<u8>) -> Result<T, Error>,
    {
        fs::read(&path)
            .map_err(|e| {
                Error::Reference(
                    language.clone(),
                    name.clone(),
                    path,
                    Box::new(e),
                )
            })
            .and_then(mapper)
    }

    /// Attempts to load a text file.
    ///
    /// This functtion will fail if the file does not contain _UTF-8_ encoded
    /// text.
    ///
    /// # Arguments
    /// *  `language` - The language for this template. This is used to
    ///    generate an error message.
    /// *  `name` - The name of this template. This is used to generate and
    ///    error message.
    /// *  `path` - The full path to the file to read.
    fn load_string(
        language: &Language,
        name: &TemplateName,
        path: PathBuf,
    ) -> Result<String, Error> {
        Self::load(language, name, path, |d| {
            String::from_utf8(d).map_err(|e| Error::Parse(Box::new(e)))
        })
    }
}

/// The file representation of a collection of templates.
#[derive(Deserialize, Serialize)]
#[serde(transparent)]
struct TemplateDescriptions(
    HashMap<Language, HashMap<TemplateName, TemplateDescription>>,
);

impl TemplateDescriptions {
    /// Loads a description of a collection of templates.
    ///
    /// # Arguments
    /// *  `path` - The source path.
    pub fn load<P>(path: P) -> Result<Self, io::Error>
    where
        P: AsRef<Path>,
    {
        toml::from_str(&fs::read_to_string(path)?)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}

impl Templates {
    /// Attempts to load a collection of templates from a description file.
    ///
    /// # Arguments
    /// *  `path` - The source path.
    pub fn load<P>(path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let parent = path
            .as_ref()
            .parent()
            .map_or_else(PathBuf::new, PathBuf::from);
        Ok(Self(
            TemplateDescriptions::load(path)
                .map_err(|e| Error::Parse(Box::new(e)))?
                .0
                .into_iter()
                .map(|(language, templates)| {
                    Ok((
                        language.clone(),
                        templates
                            .into_iter()
                            .map(|(name, template)| {
                                Ok((
                                    name.clone(),
                                    Template::new(
                                        &template.wrapping_data(
                                            &language, &name, &parent,
                                        )?,
                                        template.subject.clone(),
                                        template.body_data(
                                            &language, &name, &parent,
                                        )?,
                                        template.attachments_data(
                                            &language, &name, &parent,
                                        )?,
                                    ),
                                ))
                            })
                            .collect::<Result<HashMap<_, _>, Error>>()?,
                    ))
                })
                .collect::<Result<HashMap<_, _, _>, Error>>()?,
        ))
    }

    /// Attempts to locate a named template for a given language.
    ///
    /// # Arguments
    /// *  `language` - The requested language.
    /// *  `name` - The template name.
    pub fn get(
        &self,
        language: &Language,
        name: &TemplateName,
    ) -> Option<&Template> {
        self.0.get(language).and_then(|l| l.get(name))
    }
}

impl Template {
    /// The element in the wrapper document that is replaced with the message
    /// body when generating HTML.
    ///
    /// Although this looks like an XML element, it is in reality a verbatim
    /// string, so an exact match is required.
    pub const MESSAGE_ELEMENT: &'static str = "<message/>";

    /// Creates a new template from a wrapper and markdown source.
    ///
    /// # Arguments
    /// *  `wrapping` - The HTML wrapping the message.
    /// *  `subject` - The subject of the message.
    /// *  `body` - The markdown document.
    /// *  `attachments` - Additional attachments.
    pub fn new(
        wrapping: &str,
        subject: String,
        body: String,
        attachments: HashMap<String, Attachment>,
    ) -> Self {
        let wrapping = {
            let mut parts = wrapping.splitn(2, Self::MESSAGE_ELEMENT);
            (
                parts.next().unwrap_or("").into(),
                parts.next().unwrap_or("").into(),
            )
        };
        Self {
            wrapping,
            subject,
            body,
            attachments,
        }
    }

    /// The subject of this message.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Converts this template to HTML, given replacements from `replacements`.
    ///
    /// # Arguments
    /// *  `replacements` - A function converting keys to replacement strings.
    ///    If this function returns `None`, the replacement string is kept.
    pub fn html<'a, F>(&self, replacements: F) -> String
    where
        F: Fn(&str) -> Option<&'a str> + 'a,
    {
        let mut result =
            self.interpolate(&self.wrapping.0, |key| replacements(key));
        html::push_html(&mut result, self.events(|key| replacements(key)));
        result.push_str(
            &self.interpolate(&self.wrapping.1, |key| replacements(key)),
        );
        result
    }

    /// Converts this template to text, given replacements from `replacements`.
    ///
    /// # Arguments
    /// *  `replacements` - A function converting keys to replacement strings.
    ///    If this function returns `None`, the replacement string is kept.
    pub fn text<'a, F>(&self, replacements: F) -> String
    where
        F: Fn(&str) -> Option<&'a str> + 'a,
    {
        self.interpolate(&self.body, replacements)
    }

    /// The attachments for this message.
    pub fn attachments(&self) -> &HashMap<String, Attachment> {
        &self.attachments
    }

    /// Provides a sequence of markdown events, interpolating texts with
    /// `replacements`.
    ///
    /// # Arguments
    /// *  `replacements` - A function converting keys to replacement strings.
    ///    If this function returns `None`, the replacement string is kept.
    fn events<'a, F>(
        &'a self,
        replacements: F,
    ) -> impl Iterator<Item = Event<'a>> + 'a
    where
        F: Fn(&str) -> Option<&'a str> + 'a,
    {
        Parser::new(&self.body).map(move |event| match event {
            Event::Text(text) => {
                Event::Text(self.interpolate(&text, |k| replacements(k)).into())
            }
            Event::Code(text) => {
                Event::Code(self.interpolate(&text, |k| replacements(k)).into())
            }
            Event::Html(text) => {
                Event::Html(self.interpolate(&text, |k| replacements(k)).into())
            }
            Event::FootnoteReference(text) => Event::FootnoteReference(
                self.interpolate(&text, |k| replacements(k)).into(),
            ),
            Event::Start(Tag::FootnoteDefinition(text)) => {
                Event::Start(Tag::FootnoteDefinition(
                    self.interpolate(&text, |k| replacements(k)).into(),
                ))
            }
            Event::Start(Tag::Link(lt, url, text)) => Event::Start(Tag::Link(
                lt,
                self.interpolate(&url, |k| replacements(k)).into(),
                self.interpolate(&text, |k| replacements(k)).into(),
            )),
            Event::Start(Tag::Image(lt, url, text)) => {
                Event::Start(Tag::Image(
                    lt,
                    self.interpolate(&url, |k| replacements(k)).into(),
                    self.interpolate(&text, |k| replacements(k)).into(),
                ))
            }
            e => e,
        })
    }

    /// Interpolates all replacements in `string` given replacements in
    /// `replacements`.
    ///
    /// Tokens for which `replacements` returns `None` are kept.
    ///
    /// # Arguments
    /// *  `string` - The string to interpolate.
    /// *  `replacements` - A function converting keys to replacement strings.
    fn interpolate<'a, F>(&self, string: &str, replacements: F) -> String
    where
        F: Fn(&str) -> Option<&'a str> + 'a,
    {
        let mut text = string.to_string();
        let mut index = 0;
        while let Some((replacement_range, key_range)) =
            Self::next_replacement(index, &text)
        {
            let key = &text[key_range.clone()];
            if let Some(replacement) = replacements(key).map(str::to_string) {
                index += replacement_range.start + replacement.len();
                text = text.clone();
                text.replace_range(replacement_range, &replacement);
            } else {
                index += replacement_range.start + key.len();
            }
        }
        text
    }

    /// Finds the range to be replaced by the next replacement token, and the
    /// range of the token itself.
    ///
    /// Since a replacement token is marked with `"${token}"`, the replacement
    /// token will always be a subset of the text to be replcaed.
    ///
    /// # Arguments
    /// *  `offset` - The start offset. Characters before this will be ignored.
    /// *  `string` - The string in which to search.
    fn next_replacement(
        offset: usize,
        string: &str,
    ) -> Option<(Range<usize>, Range<usize>)> {
        enum State {
            BeforeStart,
            Start(usize),
            Key(usize, usize),
        }
        let mut state = State::BeforeStart;

        use State::*;
        for (i, c) in string.chars().enumerate().skip(offset) {
            state = match (state, c) {
                (BeforeStart, '$') => Start(i),
                (Start(p), '{') => Key(p, i + 1),
                (Key(p, k), '}') => return Some((p..i + 1, k..i)),
                (Key(p, k), _) => Key(p, k),
                _ => BeforeStart,
            };
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    #[test]
    fn templates_valid() {
        let attachments = vec![(
            String::from("file1"),
            Attachment {
                content_type: ContentType::parse("text/plain").unwrap(),
                data: fs::read(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("resources/test/email/template/attachment.txt"),
                )
                .unwrap(),
            },
        )]
        .into_iter()
        .collect();
        assert_eq!(
            Templates(
                [(
                    "l1".into(),
                    [(
                        "t1".into(),
                        Template::new(
                            "<html><body><message/></body></html>\n",
                            "subject".into(),
                            "# Header l1/t1\n\nParagraph\n\nReplaced: \
                            ${replace}\n"
                                .into(),
                            attachments,
                        ),
                    ),]
                    .iter()
                    .cloned()
                    .collect()
                ),]
                .iter()
                .cloned()
                .collect()
            ),
            Templates::load(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("resources/test/email/template/valid.toml"),
            )
            .unwrap(),
        );
    }

    #[test]
    fn text_simple_replacements() {
        assert_eq!(
            "replacement 1, replacement 2",
            Template::new(
                "",
                "subject".into(),
                "${r1}, ${r2}".into(),
                Default::default(),
            )
            .text(|r| match r {
                "r1" => Some(&"replacement 1"),
                "r2" => Some(&"replacement 2"),
                _ => None,
            }),
        );
    }

    #[test]
    fn html_no_replacements() {
        assert_eq!(
            "<h1>Header</h1>\n<p>This is the <em>first</em> paragraph.\
            </p>\n<p>And <em>this</em> is the second.</p>\n",
            Template::new(
                "",
                "subject".into(),
                r"# Header

This is the *first* paragraph.

And _this_ is the second."
                    .into(),
                Default::default(),
            )
            .html(|_| None),
        );
    }

    #[test]
    fn html_simple_replacements() {
        assert_eq!(
            "<p>replacement 1, replacement 2</p>\n",
            Template::new(
                "",
                "subject".into(),
                "${r1}, ${r2}".into(),
                Default::default(),
            )
            .html(|r| match r {
                "r1" => Some(&"replacement 1"),
                "r2" => Some(&"replacement 2"),
                _ => None,
            }),
        );
    }

    #[test]
    fn html_simple_replacements_with_wrapper() {
        assert_eq!(
            "<html><body><p>replacement 1, replacement 2</p>\n</body></html>",
            Template::new(
                "<html><body><message/></body></html>",
                "subject".into(),
                "${r1}, ${r2}".into(),
                Default::default(),
            )
            .html(|r| match r {
                "r1" => Some(&"replacement 1"),
                "r2" => Some(&"replacement 2"),
                _ => None,
            }),
        );
    }

    #[test]
    fn html_wrapper_replacements() {
        assert_eq!(
            "<header>r</header><p>r</p>\n<footer>r</footer>",
            Template::new(
                "<header>${r}</header><message/><footer>${r}</footer>".into(),
                "subject".into(),
                "${r}".into(),
                Default::default(),
            )
            .html(|r| match r {
                "r" => Some(&"r"),
                _ => None,
            }),
        );
    }

    #[test]
    fn html_code_replacements() {
        assert_eq!(
            "<p><code>replacement 1, replacement 2</code></p>\n",
            Template::new(
                "",
                "subject".into(),
                "`${r1}, ${r2}`".into(),
                Default::default(),
            )
            .html(|r| match r {
                "r1" => Some(&"replacement 1"),
                "r2" => Some(&"replacement 2"),
                _ => None,
            }),
        );
    }

    #[test]
    fn html_href_replacements() {
        assert_eq!(
            "<p><a href=\"http://example.com\">link</a></p>\n",
            Template::new(
                "",
                "subject".into(),
                "[link](${r})".into(),
                Default::default(),
            )
            .html(|r| match r {
                "r" => Some(&"http://example.com"),
                _ => None,
            }),
        );
    }

    #[test]
    fn html_broken_replacements() {
        assert_eq!(
            "<p>${r</p>\n",
            Template::new(
                "",
                "subject".into(),
                "${r".into(),
                Default::default(),
            )
            .html(|r| match r {
                "r" => Some(&"this & *that*"),
                _ => None,
            }),
        );
    }

    #[test]
    fn html_missing_replacements() {
        assert_eq!(
            "<p>replacement 1, ${r2}</p>\n",
            Template::new(
                "",
                "subject".into(),
                "${r1}, ${r2}".into(),
                Default::default(),
            )
            .html(|r| match r {
                "r1" => Some(&"replacement 1"),
                _ => None,
            }),
        );
    }

    #[test]
    fn html_escaped_replacements() {
        assert_eq!(
            "<p>this &amp; *that*</p>\n",
            Template::new(
                "",
                "subject".into(),
                "${r}".into(),
                Default::default(),
            )
            .html(|r| match r {
                "r" => Some(&"this & *that*"),
                _ => None,
            }),
        );
    }

    #[test]
    fn html_long_replacement_key() {
        assert_eq!(
            "<p>This example is correct</p>\n",
            Template::new(
                "",
                "subject".into(),
                "This example ${uses a very, VERY long replacement token}"
                    .into(),
                Default::default(),
            )
            .html(|r| match r {
                "uses a very, VERY long replacement token" =>
                    Some(&"is correct"),
                _ => None,
            }),
        );
    }
}
