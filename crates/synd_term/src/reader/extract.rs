use anyhow::{Context as _, Result, ensure};
use dom_smoothie::Readability;
use url::Url;

use super::Article;

/// Extract the article using the final response URL to resolve relative links.
pub(crate) fn extract(html: &str, url: &Url) -> Result<Article> {
    let article = Readability::new(html, Some(url.as_str()), None)?
        .extract()
        .context("could not extract the article body")?;
    let markdown = article.root().md(None);
    ensure!(!markdown.trim().is_empty(), "the article body is empty");
    let title = article.metadata.title.trim();
    Ok(Article {
        title: if title.is_empty() {
            url.as_str()
        } else {
            title
        }
        .to_owned(),
        markdown: markdown.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn article_links_resolve_against_the_publisher_page() {
        let url = Url::parse("https://example.org/journal/issue/").unwrap();
        let article = extract(
            r#"<article><p>The release is available now. Read the
            <a href="../notes">release notes</a> for the details.</p></article>"#,
            &url,
        )
        .unwrap();

        assert!(
            article
                .markdown
                .contains("[release notes](https://example.org/journal/notes)")
        );
    }
}
