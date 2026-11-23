use synd_feed::{entry::Entry, types::FeedMeta};
use url::form_urlencoded;

use crate::{
    db::FeedDb,
    event::ProcessorResult,
    feed::pipeline::{Ctx, EntryStage},
};

pub(in crate::feed::pipeline) struct RemoveQueryParams {
    matchers: Vec<ParameterPolicy>,
    exclusions: Vec<UrlPolicy>,
}

impl Default for RemoveQueryParams {
    fn default() -> Self {
        use {ParameterPolicy::UtmParams, UrlPolicy::SigUrls};

        Self::new().matches(UtmParams).exclude(SigUrls)
    }
}

pub(in crate::feed::pipeline) enum ParameterPolicy {
    UtmParams,
}

impl ParameterPolicy {
    fn matches(&self, name: &str) -> bool {
        match self {
            Self::UtmParams => name.starts_with("utm_"),
        }
    }
}

pub(in crate::feed::pipeline) enum UrlPolicy {
    SigUrls,
}

impl UrlPolicy {
    fn matches(&self, query: &str) -> bool {
        match self {
            Self::SigUrls => has_known_signature(query),
        }
    }
}

impl EntryStage for RemoveQueryParams {
    type Input = Entry;
    type Output = Entry;

    async fn run_entry<Tx: FeedDb + Send>(
        &self,
        mut entry: Entry,
        _: &FeedMeta,
        _: &mut Ctx<'_, Tx>,
    ) -> ProcessorResult<Option<Entry>> {
        for link in entry.links_mut() {
            self.apply_to_href(link.href_mut());
        }
        Ok(Some(entry))
    }
}

impl RemoveQueryParams {
    pub fn new() -> Self {
        Self {
            matchers: Vec::new(),
            exclusions: Vec::new(),
        }
    }

    pub fn matches(mut self, policy: ParameterPolicy) -> Self {
        self.matchers.push(policy);
        self
    }

    pub fn exclude(mut self, policy: UrlPolicy) -> Self {
        self.exclusions.push(policy);
        self
    }

    fn apply_to_href(&self, href: &mut String) {
        let query_end = href.find('#').unwrap_or(href.len());
        let Some(query_start) = href[..query_end].find('?') else {
            return;
        };
        let query = &href[query_start + 1..query_end];
        let matches = |parameter: &str| {
            form_urlencoded::parse(parameter.as_bytes())
                .next()
                .is_some_and(|(name, _)| self.matchers.iter().any(|policy| policy.matches(&name)))
        };
        let parameters = query.split('&');
        if !parameters.clone().any(matches)
            || self.exclusions.iter().any(|policy| policy.matches(query))
        {
            return;
        }

        // Keep retained parameters verbatim, including duplicates and their encoding.
        let retained = parameters
            .filter(|parameter| !matches(parameter))
            .collect::<Vec<_>>()
            .join("&");
        if retained.is_empty() {
            href.replace_range(query_start..query_end, "");
        } else {
            href.replace_range(query_start + 1..query_end, &retained);
        }
    }
}

fn has_known_signature(query: &str) -> bool {
    let mut signature = false;
    let mut key_pair_id = false;
    for (name, _) in form_urlencoded::parse(query.as_bytes()) {
        match name.as_ref() {
            "X-Amz-Signature" | "X-Goog-Signature" => return true,
            "Signature" => signature = true,
            "Key-Pair-Id" => key_pair_id = true,
            _ => {}
        }
    }
    signature && key_pair_id // CloudFront
}

#[cfg(test)]
mod tests {
    use super::{ParameterPolicy::UtmParams, RemoveQueryParams};

    #[test]
    fn custom_stage_can_omit_signature_exclusion() {
        let mut url = "/file?utm_source=rss&X-Amz-Signature=abc".to_owned();
        RemoveQueryParams::new()
            .matches(UtmParams)
            .apply_to_href(&mut url);
        assert_eq!(url, "/file?X-Amz-Signature=abc");
    }

    #[test]
    fn preserves_other_parameters_and_the_fragment_verbatim() {
        let mut url =
            "/post?flag&q=x%20y&%75tm_source=rss&q=%2f&utm_medium#part?utm_source=keep".to_owned();
        RemoveQueryParams::default().apply_to_href(&mut url);
        assert_eq!(url, "/post?flag&q=x%20y&q=%2f#part?utm_source=keep");
    }

    #[test]
    fn keeps_urls_without_matching_parameters_verbatim() {
        for original in [
            "/post?",
            "/post#part?utm_source=rss",
            "/post?q=utm_campaign",
        ] {
            let mut url = original.to_owned();
            RemoveQueryParams::default().apply_to_href(&mut url);
            assert_eq!(url, original);
        }
    }

    #[test]
    fn preserves_known_signed_urls_verbatim() {
        for original in [
            "/file?utm_source=rss&X-Amz-Signature=abc",
            "/file?X-Goog-Signature=def&utm_medium=feed",
            "/file?Key-Pair-Id=K1&utm_source=rss&Signature=ghi",
        ] {
            let mut url = original.to_owned();
            RemoveQueryParams::default().apply_to_href(&mut url);
            assert_eq!(url, original);
        }
    }

    #[test]
    fn cloudfront_requires_both_signature_and_key_pair_id() {
        for parameter in ["Signature=abc", "Key-Pair-Id=K1", "q=X-Amz-Signature"] {
            let mut url = format!("/file?{parameter}&utm_source=rss");
            RemoveQueryParams::default().apply_to_href(&mut url);
            assert_eq!(url, format!("/file?{parameter}"));
        }
    }
}
