use async_graphql::{Enum, SimpleObject};
use synd_feed::entry::Entry;
use synd_support::html::{ExcerptRequest, HtmlConvError, HtmlConverter};

#[derive(Clone, Copy, Enum, Eq, PartialEq)]
enum ExcerptSource {
    Summary,
    Content,
}

#[derive(SimpleObject)]
pub(super) struct EntryExcerpt {
    source: ExcerptSource,
    text: String,
}

impl EntryExcerpt {
    pub(super) fn from_entry(
        entry: &Entry,
        max_unicode_width: usize,
        converter: &dyn HtmlConverter,
    ) -> Result<Option<Self>, HtmlConvError> {
        let summary = entry.summary().map(|summary| {
            (
                ExcerptSource::Summary,
                summary.content(),
                summary.content_type(),
            )
        });
        let content = entry.content().and_then(|content| {
            Some((
                ExcerptSource::Content,
                content.body()?,
                content.content_type(),
            ))
        });

        for (source, body, media_type) in summary.into_iter().chain(content) {
            let request = ExcerptRequest::builder()
                .content(body)
                .media_type(media_type)
                .max_unicode_width(max_unicode_width)
                .build();
            let excerpt = match converter.excerpt(request) {
                Err(HtmlConvError::NoReadableText) => continue,
                result => result?,
            };
            return Ok(Some(Self {
                source,
                text: excerpt.into_text(),
            }));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_graphql::{EmptyMutation, EmptySubscription, Schema, value};
    use synd_feed::{
        entry::{Content, EntryId},
        types::{Annotated, FeedMeta, FeedType, FeedUrl, Text},
    };
    use synd_support::html::{Article, ArticleRequest, DefaultHtmlConverter, Excerpt};

    use super::{Entry, ExcerptRequest, HtmlConvError, HtmlConverter};
    use crate::gql::object;

    fn schema(
        summary: Option<&str>,
        content: Option<(&str, &str)>,
        converter: Arc<dyn HtmlConverter>,
    ) -> Schema<object::Entry, EmptyMutation, EmptySubscription> {
        let entry = Entry::builder()
            .id(EntryId::parse(format!("synd:entry:v1:{}", "0".repeat(64))).unwrap())
            .maybe_summary(summary.map(|body| {
                Text::builder()
                    .content(body.into())
                    .content_type("text/html".into())
                    .build()
            }))
            .maybe_content(content.map(|(body, media_type)| {
                Content::builder()
                    .body(body.into())
                    .content_type(media_type.into())
                    .build()
            }))
            .build();
        let meta = FeedMeta::builder()
            .url(FeedUrl::parse("https://example.com/feed").unwrap())
            .feed_type(FeedType::Atom)
            .build();
        Schema::build(
            object::Entry::new(Annotated::new(meta), entry),
            EmptyMutation,
            EmptySubscription,
        )
        .data(converter)
        .finish()
    }

    #[tokio::test]
    async fn bounds_a_content_only_patch_after_decoding_html() {
        let patch = format!("<pre><span>Aあ&amp;👩‍💻</span>{}</pre>", "+".repeat(2_000));
        let schema = schema(
            None,
            Some((&patch, "text/html; charset=utf-8")),
            Arc::new(DefaultHtmlConverter),
        );
        let response = schema
            .execute(
                "{
            excerpt { source text }
            feedSummary { content }
            content { body contentType }
            short: excerpt(maxUnicodeWidth: 5) { text }
            max: excerpt(maxUnicodeWidth: 1000) { text }
        }",
            )
            .await;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        assert_eq!(
            response.data,
            value!({
                "excerpt": { "source": "CONTENT", "text": format!("Aあ&👩‍💻{}", "+".repeat(194)) },
                "feedSummary": null,
                "content": { "body": patch, "contentType": "text/html; charset=utf-8" },
                "short": { "text": "Aあ&" },
                "max": { "text": format!("Aあ&👩‍💻{}", "+".repeat(994)) },
            })
        );
        let errors = schema
            .execute("{ excerpt(maxUnicodeWidth: 1001) { text } }")
            .await
            .into_result()
            .unwrap_err();
        assert!(
            errors[0].message.contains("less than or equal to 1000"),
            "{errors:?}"
        );
        let errors = schema
            .execute("{ summary }")
            .await
            .into_result()
            .unwrap_err();
        assert!(
            errors[0].message.contains("Unknown field \"summary\""),
            "{errors:?}"
        );
    }

    #[tokio::test]
    async fn prefers_readable_summary_and_preserves_plain_text_content() {
        for (summary, body, expected) in [
            (
                Some("<p>Short &amp; <b>sweet</b></p>"),
                Some(("<different>", "text/plain")),
                value!({ "source": "SUMMARY", "text": "Short & sweet" }),
            ),
            (
                Some("<p> </p><script>hidden</script>"),
                Some(("Vec<T> & U", "text/plain")),
                value!({ "source": "CONTENT", "text": "Vec<T> & U" }),
            ),
            (None, None, value!(null)),
        ] {
            let response = schema(summary, body, Arc::new(DefaultHtmlConverter))
                .execute("{ excerpt { source text } }")
                .await;
            assert!(response.errors.is_empty(), "{:?}", response.errors);
            assert_eq!(response.data, value!({ "excerpt": expected }));
        }
    }

    #[tokio::test]
    async fn zero_width_keeps_the_readable_source() {
        let response = schema(
            Some("あ"),
            Some(("body", "text/plain")),
            Arc::new(DefaultHtmlConverter),
        )
        .execute("{ excerpt(maxUnicodeWidth: 0) { source text } }")
        .await
        .into_result()
        .unwrap();
        assert_eq!(
            response.data,
            value!({ "excerpt": { "source": "SUMMARY", "text": "" } })
        );
    }

    #[tokio::test]
    async fn conversion_failure_is_reported_without_trying_the_next_source() {
        struct FailingConverter;
        impl HtmlConverter for FailingConverter {
            fn excerpt(&self, request: ExcerptRequest<'_>) -> Result<Excerpt, HtmlConvError> {
                assert_eq!(request.content(), "summary");
                Err(HtmlConvError::Conversion(
                    std::io::Error::other("test conversion failure").into(),
                ))
            }

            fn extract_article(&self, _: ArticleRequest<'_>) -> Result<Article, HtmlConvError> {
                unreachable!()
            }
        }
        let errors = schema(
            Some("summary"),
            Some(("body", "text/plain")),
            Arc::new(FailingConverter),
        )
        .execute("{ excerpt { text } }")
        .await
        .into_result()
        .unwrap_err();
        let error = errors[0].source::<HtmlConvError>().unwrap();
        assert_eq!(
            std::error::Error::source(error).unwrap().to_string(),
            "test conversion failure"
        );
    }
}
