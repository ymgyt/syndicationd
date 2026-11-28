use crate::{config::Categories, ui::widgets::prompt::Prompt};
use serde_json::Value;
use synd_client::payload::{
    CrawlPolicyInput, PollingIntervalSeconds, PollingPolicy, PollingPolicyInput, SubscribeFeedInput,
};
use synd_feed::types::{Category, FeedUrl, SubscriptionLabels};

use super::Field;

/// Editable subscription values and their conversion to the API input.
pub(super) struct Draft {
    pub(super) url: Prompt,
    pub(super) labels: Vec<LabelRow>,
    pub(super) poll_every: Prompt,
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            url: Prompt::default(),
            labels: vec![LabelRow {
                key: "category".into(),
                ..LabelRow::default()
            }],
            poll_every: Prompt::default(),
        }
    }
}

#[derive(Default)]
pub(super) struct LabelRow {
    pub(super) key: Prompt,
    pub(super) value: Prompt,
    // Keep untouched labels verbatim, including legacy keys and unnormalized categories.
    original: Option<(String, Value)>,
}

impl LabelRow {
    fn displayed_value(key: &str, value: &Value) -> String {
        let Value::String(text) = value else {
            return value.to_string();
        };
        if text.chars().any(char::is_control) {
            return value.to_string();
        }
        if key == "category" || Self::parse_value(text).is_ok_and(|parsed| &parsed == value) {
            return text.clone();
        }
        value.to_string()
    }

    fn parse_value(raw: &str) -> Result<Value, serde_json::Error> {
        match serde_json::from_str(raw) {
            Ok(value) => Ok(value),
            Err(error) if raw.trim_start().starts_with(['{', '[', '"']) => Err(error),
            Err(_) => Ok(Value::String(raw.to_owned())),
        }
    }

    fn from_label(key: &str, value: &Value) -> Self {
        Self {
            key: key.into(),
            value: Self::displayed_value(key, value).as_str().into(),
            original: Some((key.to_owned(), value.clone())),
        }
    }

    pub(super) fn entry(
        &self,
        index: usize,
        categories: &Categories,
    ) -> Result<Option<(&str, Value)>, (Field, String)> {
        if let Some((key, value)) = &self.original
            && self.key.line() == key
            && self.value.line() == Self::displayed_value(key, value)
        {
            return Ok(Some((key, value.clone())));
        }
        let key = self.key.line().trim();
        let raw = self.value.line();
        if key.is_empty() {
            return Err((
                Field::Key(index),
                "Enter a label key or remove this label.".into(),
            ));
        }
        let value = if key == "category" {
            if raw.trim().is_empty() {
                return Ok(None);
            }
            let category = Category::new(raw.to_owned()).map_err(|_| {
                (
                    Field::Value(index),
                    format!(
                        "Use at most {} bytes for a category name.",
                        Category::MAX_LEN
                    ),
                )
            })?;
            Value::String(categories.normalize(category).into_inner().into_owned())
        } else {
            Self::parse_value(raw)
                .map_err(|err| (Field::Value(index), format!("Enter a JSON value: {err}")))?
        };
        Ok(Some((key, value)))
    }
}

pub(super) fn interval_text(seconds: u64) -> String {
    if seconds.is_multiple_of(3600) {
        format!("{}h", seconds / 3600)
    } else if seconds.is_multiple_of(60) {
        format!("{}m", seconds / 60)
    } else {
        format!("{seconds}s")
    }
}

fn polling_interval(text: &str) -> Option<PollingIntervalSeconds> {
    let duration = synd_support::time::humantime::parse_duration(text).ok()?;
    if duration.subsec_nanos() != 0 {
        return None;
    }
    i64::try_from(duration.as_secs()).ok()?.try_into().ok()
}

impl Draft {
    pub(super) fn edit(
        url: &FeedUrl,
        polling: &PollingPolicy,
        labels: &SubscriptionLabels,
    ) -> Self {
        let interval = match polling {
            PollingPolicy::Manual => "manual".into(),
            PollingPolicy::Interval { seconds } => interval_text(seconds.get().cast_unsigned()),
            PollingPolicy::Other { kind, .. } => format!("Unsupported ({kind})"),
        };
        Self {
            url: url.as_str().into(),
            poll_every: interval.as_str().into(),
            labels: labels
                .iter()
                .map(|(key, value)| LabelRow::from_label(key, value))
                .collect(),
        }
    }

    pub(super) fn prompt_mut(&mut self, field: Field) -> Option<&mut Prompt> {
        match field {
            Field::Url => Some(&mut self.url),
            Field::Label(i) | Field::Value(i) => Some(&mut self.labels[i].value),
            Field::Key(i) => Some(&mut self.labels[i].key),
            Field::PollEvery => Some(&mut self.poll_every),
            _ => None,
        }
    }

    pub(super) fn input(
        &self,
        categories: &Categories,
    ) -> Result<SubscribeFeedInput, (Field, String)> {
        let url = FeedUrl::parse(self.url.line().trim()).map_err(|_| {
            (
                Field::Url,
                "Enter a feed URL, e.g. https://example.org/feed.xml".into(),
            )
        })?;
        let crawl_policy = self.crawl_policy()?;
        let mut labels = SubscriptionLabels::default();
        for (i, row) in self.labels.iter().enumerate() {
            if let Some((key, value)) = row.entry(i, categories)?
                && labels.insert(key, value).is_some()
            {
                return Err((Field::Key(i), format!("Duplicate label key: {key}")));
            }
        }
        Ok(SubscribeFeedInput {
            url,
            labels,
            crawl_policy,
        })
    }

    fn crawl_policy(&self) -> Result<Option<CrawlPolicyInput>, (Field, String)> {
        let interval = self.poll_every.line().trim();
        if interval.is_empty() || interval.eq_ignore_ascii_case("default") {
            return Ok(None);
        }
        if interval.eq_ignore_ascii_case("manual") {
            return Ok(Some(CrawlPolicyInput {
                polling: PollingPolicyInput::Manual,
            }));
        }
        let seconds = polling_interval(interval).ok_or_else(|| {
            (
                Field::PollEvery,
                "Use manual or a duration in whole seconds of at least 1s, e.g. 30m.".into(),
            )
        })?;
        Ok(Some(CrawlPolicyInput {
            polling: PollingPolicyInput::Interval { seconds },
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn changing_category_preserves_the_existing_interval_and_json_labels() {
        let url = FeedUrl::parse("https://example.org/feed").unwrap();
        let seconds = PollingIntervalSeconds::try_from(7_200).unwrap();
        let mut labels: SubscriptionLabels = serde_json::from_value(json!({
            "category": "rust", "active": false, "id": "123", "tags": ["rust"], " legacy ": null
        }))
        .unwrap();
        let mut draft = Draft::edit(&url, &PollingPolicy::Interval { seconds }, &labels);
        assert_eq!(draft.poll_every.line(), "2h");
        draft
            .labels
            .iter_mut()
            .find(|row| row.key.line() == "category")
            .unwrap()
            .value = "k8s".into();
        let input = draft.input(&Categories::default_toml()).unwrap();
        labels.insert("category", json!("kubernetes"));
        assert_eq!(input.labels, labels);
        assert_eq!(
            input.crawl_policy,
            Some(CrawlPolicyInput {
                polling: PollingPolicyInput::Interval { seconds }
            })
        );
    }

    #[test]
    fn infers_label_values_and_keeps_category_names_as_text() {
        let draft = Draft {
            url: "https://example.org/feed.xml".into(),
            labels: [
                ("priority", "3"),
                ("code", r#""3""#),
                ("issue", "003"),
                ("topic", "rust"),
                ("archived", "false"),
                ("tags", r#"["rust", "releases"]"#),
                ("category", "2026"),
            ]
            .into_iter()
            .map(|(key, value)| LabelRow {
                key: key.into(),
                value: value.into(),
                ..LabelRow::default()
            })
            .collect(),
            ..Draft::default()
        };
        let input = draft.input(&Categories::default_toml()).unwrap();
        assert_eq!(
            serde_json::to_value(input.labels).unwrap(),
            json!({
                "priority": 3, "code": "3", "issue": "003", "topic": "rust",
                "archived": false, "tags": ["rust", "releases"], "category": "2026"
            })
        );
    }
}
