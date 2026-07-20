use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synd_client::payload;
use synd_feed::types::{Category, FeedType, FeedUrl, Requirement};
use tracing::warn;

use crate::ui;

mod time;
pub use time::{Time, TimeExt};

pub use synd_client::payload::{EntryMeta, Link, PageInfo};

mod requirement_ext;
pub use requirement_ext::RequirementExt;

pub(crate) mod github;

pub trait EntryMetaExt {
    fn summary_text(&self, width: usize) -> Option<String>;
}

impl EntryMetaExt for EntryMeta {
    fn summary_text(&self, width: usize) -> Option<String> {
        self.summary.as_deref().and_then(|summary| {
            match html2text::from_read(summary.as_bytes(), width) {
                Ok(text) => Some(text),
                Err(err) => {
                    warn!("convert summary html to text: {err}");
                    None
                }
            }
        })
    }
}

pub trait CrawlPolicyExt {
    fn prompt_value(&self) -> Option<String>;
}

impl CrawlPolicyExt for payload::CrawlPolicy {
    fn prompt_value(&self) -> Option<String> {
        self.polling.prompt_value()
    }
}

trait PollingPolicyExt {
    fn prompt_value(&self) -> Option<String>;
}

impl PollingPolicyExt for payload::PollingPolicy {
    fn prompt_value(&self) -> Option<String> {
        match self.kind {
            payload::PollingPolicyKind::Manual => Some("manual".to_owned()),
            payload::PollingPolicyKind::Interval => self
                .interval_seconds
                .filter(|seconds| *seconds > 0)
                .map(|seconds| format!("interval:{seconds}s")),
            payload::PollingPolicyKind::Other(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(fake::Dummy))]
pub struct Feed {
    pub feed_type: Option<FeedType>,
    pub title: Option<String>,
    pub url: FeedUrl,
    pub updated: Option<Time>,
    pub links: Vec<Link>,
    pub website_url: Option<String>,
    pub description: Option<String>,
    pub generator: Option<String>,
    pub entries: Vec<EntryMeta>,
    pub authors: Vec<String>,
    pub crawl_policy: payload::CrawlPolicy,
    requirement: Option<Requirement>,
    category: Option<Category<'static>>,
}

impl Feed {
    pub fn requirement(&self) -> Requirement {
        self.requirement.unwrap_or(ui::DEFAULT_REQUIREMENT)
    }

    pub fn category(&self) -> &Category<'static> {
        self.category.as_ref().unwrap_or(ui::default_category())
    }

    #[must_use]
    pub fn with_url(self, url: FeedUrl) -> Self {
        Self { url, ..self }
    }

    #[must_use]
    pub fn with_requirement(self, requirement: Requirement) -> Self {
        Self {
            requirement: Some(requirement),
            ..self
        }
    }

    #[must_use]
    pub fn with_category(self, category: Category<'static>) -> Self {
        Self {
            category: Some(category),
            ..self
        }
    }
}

impl From<payload::SubscribedFeed> for Feed {
    fn from(f: payload::SubscribedFeed) -> Self {
        let payload::SubscribedFeed {
            url,
            requirement,
            category,
            crawl_policy,
            feed: details,
        } = f;
        Self {
            feed_type: details
                .as_ref()
                .and_then(|details| details.feed_type.clone().into_feed_type()),
            title: details.as_ref().and_then(|details| details.title.clone()),
            url,
            updated: details.as_ref().and_then(|details| details.updated),
            links: details
                .as_ref()
                .map(|details| details.links.nodes.clone())
                .unwrap_or_default(),
            website_url: details
                .as_ref()
                .and_then(|details| details.website_url.clone()),
            description: details
                .as_ref()
                .and_then(|details| details.description.clone()),
            generator: details
                .as_ref()
                .and_then(|details| details.generator.clone()),
            entries: details
                .as_ref()
                .map(|details| details.entries.nodes.clone())
                .unwrap_or_default(),
            authors: details
                .as_ref()
                .map(|details| details.authors.nodes.clone())
                .unwrap_or_default(),
            crawl_policy,
            requirement,
            category,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExportedFeed {
    pub title: Option<String>,
    pub url: FeedUrl,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    #[serde(default)]
    pub crawl_policy: Option<ExportedCrawlPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportedCrawlPolicy {
    pub polling: ExportedPollingPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportedPollingPolicy {
    pub kind: ExportedPollingPolicyKind,
    pub interval_seconds: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExportedPollingPolicyKind {
    Manual,
    Interval,
}

impl ExportedCrawlPolicy {
    fn from_api(value: &payload::CrawlPolicy) -> Option<Self> {
        match &value.polling.kind {
            payload::PollingPolicyKind::Manual => Some(Self {
                polling: ExportedPollingPolicy {
                    kind: ExportedPollingPolicyKind::Manual,
                    interval_seconds: None,
                },
            }),
            payload::PollingPolicyKind::Interval => Some(Self {
                polling: ExportedPollingPolicy {
                    kind: ExportedPollingPolicyKind::Interval,
                    interval_seconds: value.polling.interval_seconds,
                },
            }),
            payload::PollingPolicyKind::Other(_) => None,
        }
    }
}

impl From<ExportedCrawlPolicy> for payload::CrawlPolicyInput {
    fn from(value: ExportedCrawlPolicy) -> Self {
        Self {
            polling: payload::PollingPolicyInput {
                kind: match value.polling.kind {
                    ExportedPollingPolicyKind::Manual => payload::PollingPolicyInputKind::Manual,
                    ExportedPollingPolicyKind::Interval => {
                        payload::PollingPolicyInputKind::Interval
                    }
                },
                interval_seconds: value.polling.interval_seconds,
            },
        }
    }
}

impl From<payload::SubscribedFeed> for ExportedFeed {
    fn from(v: payload::SubscribedFeed) -> Self {
        let crawl_policy = ExportedCrawlPolicy::from_api(&v.crawl_policy);
        Self {
            title: v.feed.and_then(|feed| feed.title),
            url: v.url,
            requirement: v.requirement,
            category: v.category,
            crawl_policy,
        }
    }
}

impl From<ExportedFeed> for synd_client::payload::SubscribeFeedInput {
    fn from(feed: ExportedFeed) -> Self {
        Self {
            url: feed.url,
            requirement: feed.requirement,
            category: feed.category,
            crawl_policy: feed.crawl_policy.map(Into::into),
        }
    }
}

pub trait EntryExt {
    fn summary_text(&self, width: usize) -> Option<String>;
    fn requirement(&self) -> Requirement;
    fn category(&self) -> &Category<'static>;
}

impl EntryExt for payload::Entry {
    fn summary_text(&self, width: usize) -> Option<String> {
        self.summary.as_deref().map(|summary| {
            html2text::config::plain()
                .string_from_read(summary.as_bytes(), width)
                .unwrap_or_default()
        })
    }

    fn requirement(&self) -> Requirement {
        self.feed.requirement.unwrap_or(ui::DEFAULT_REQUIREMENT)
    }

    fn category(&self) -> &Category<'static> {
        self.feed
            .category
            .as_ref()
            .unwrap_or_else(|| ui::default_category())
    }
}
