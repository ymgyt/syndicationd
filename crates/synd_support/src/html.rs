//! Convert feed text to excerpts and publisher pages to readable articles.

use std::error::Error;

use bon::Builder;
use dom_query::Document;
use dom_smoothie::Readability;
use unicode_segmentation::UnicodeSegmentation as _;
use unicode_width::UnicodeWidthStr as _;
use url::Url;

#[cfg(feature = "html-fetch")]
pub mod fetch;

/// Synchronous conversion of a feed fragment or a fetched publisher page.
pub trait HtmlConverter: Send + Sync {
    /// Return a plain-text prefix bounded by Unicode display width. A readable input
    /// can yield an empty excerpt if the first grapheme exceeds the requested width.
    fn excerpt(&self, request: ExcerptRequest<'_>) -> Result<Excerpt, HtmlConvError>;

    /// Extract the publisher's article title and Markdown, resolving links against the final URL.
    fn extract_article(&self, request: ArticleRequest<'_>) -> Result<Article, HtmlConvError>;
}

#[derive(Debug, thiserror::Error)]
pub enum HtmlConvError {
    /// The input contains no readable text, independently of any excerpt width limit.
    #[error("no readable text")]
    NoReadableText,
    /// The underlying converter failed. Its error is available via `Error::source`.
    #[error("could not convert HTML")]
    Conversion(#[source] Box<dyn Error + Send + Sync>),
}

#[derive(Debug, Builder)]
pub struct ExcerptRequest<'a> {
    content: &'a str,
    media_type: &'a str,
    max_unicode_width: usize,
}

impl<'a> ExcerptRequest<'a> {
    pub fn content(&self) -> &'a str {
        self.content
    }

    pub fn media_type(&self) -> &'a str {
        self.media_type
    }

    pub fn max_unicode_width(&self) -> usize {
        self.max_unicode_width
    }
}

#[derive(Debug)]
pub struct Excerpt {
    text: String,
}

impl Excerpt {
    pub fn new(text: String) -> Self {
        Self { text }
    }

    pub fn into_text(self) -> String {
        self.text
    }
}

#[derive(Debug, Builder)]
pub struct ArticleRequest<'a> {
    html: &'a str,
    /// Final response URL, used to resolve relative links in the page.
    base_url: &'a Url,
}

impl<'a> ArticleRequest<'a> {
    pub fn html(&self) -> &'a str {
        self.html
    }

    pub fn base_url(&self) -> &'a Url {
        self.base_url
    }
}

#[derive(Debug)]
pub struct Article {
    title: String,
    markdown: String,
}

impl Article {
    pub fn new(title: String, markdown: String) -> Self {
        Self { title, markdown }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn markdown(&self) -> &str {
        &self.markdown
    }

    pub fn into_markdown(self) -> String {
        self.markdown
    }
}

#[derive(Debug, Default)]
pub struct DefaultHtmlConverter;

impl HtmlConverter for DefaultHtmlConverter {
    fn excerpt(&self, request: ExcerptRequest<'_>) -> Result<Excerpt, HtmlConvError> {
        let media_type = request
            .media_type
            .split(';')
            .next()
            .unwrap_or_default()
            .trim();
        let formatted;
        let text = if ["text/html", "application/xhtml", "application/xhtml+xml"]
            .iter()
            .any(|html| media_type.eq_ignore_ascii_case(html))
        {
            let document = Document::fragment(request.content);
            document.select("head, script, style, template").remove();
            formatted = document.formatted_text();
            formatted.as_ref()
        } else {
            request.content
        };
        if text.trim().is_empty() {
            return Err(HtmlConvError::NoReadableText);
        }
        let text = prefix_with_width(text.trim_matches(['\r', '\n']), request.max_unicode_width);
        Ok(Excerpt::new(text.to_owned()))
    }

    fn extract_article(&self, request: ArticleRequest<'_>) -> Result<Article, HtmlConvError> {
        let base_url = request.base_url.as_str();
        let article = Readability::new(request.html, Some(base_url), None)
            .and_then(Readability::extract)
            .map_err(|source| HtmlConvError::Conversion(Box::new(source)))?;
        let markdown = article.root().md(None);
        if markdown.trim().is_empty() {
            return Err(HtmlConvError::NoReadableText);
        }
        let title = article.metadata.title.trim();
        let title = if title.is_empty() { base_url } else { title };
        Ok(Article::new(title.to_owned(), markdown.into()))
    }
}

fn prefix_with_width(text: &str, max_width: usize) -> &str {
    if max_width == 0 {
        return "";
    }
    let mut width = 0;
    for (start, grapheme) in text.grapheme_indices(true) {
        width += grapheme.width();
        if width > max_width {
            // Ligatures such as Arabic Lam-Alef can occupy one column across two graphemes.
            width = text[..start + grapheme.len()].width();
            if width > max_width {
                return &text[..start];
            }
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_graphemes_and_measures_ligatures_across_grapheme_boundaries() {
        for (content, width, expected) in [
            ("か\u{3099}👩‍💻!", 3, "か\u{3099}"),
            ("لا!", 1, "لا"),
            ("あ", 0, ""),
        ] {
            let request = ExcerptRequest::builder()
                .content(content)
                .media_type("text/plain")
                .max_unicode_width(width)
                .build();
            assert_eq!(
                DefaultHtmlConverter.excerpt(request).unwrap().into_text(),
                expected
            );
        }
    }

    #[test]
    fn article_links_resolve_against_the_publisher_page() {
        let url = Url::parse("https://example.org/journal/issue/").unwrap();
        let request = ArticleRequest::builder()
            .html(
                r#"<article><p>The release is available now. Read the
            <a href="../notes">release notes</a> for the details.</p></article>"#,
            )
            .base_url(&url)
            .build();
        let article = DefaultHtmlConverter.extract_article(request).unwrap();
        assert!(
            article
                .markdown()
                .contains("[release notes](https://example.org/journal/notes)")
        );
    }
}
