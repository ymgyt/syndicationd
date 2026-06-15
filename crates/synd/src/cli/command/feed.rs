use std::{
    io,
    process::ExitCode,
    time::{Duration, Instant},
};

use clap::{Args, Subcommand};
use synd_client::{
    Client,
    payload::{FeedEvent, SubscribeFeedInput},
};
use synd_feed::types::{Category, FeedUrl, Requirement};
use tokio::sync::mpsc;

use crate::{
    cli::{command::CommandFailure, port::PortContext},
    config::ConfigResolver,
};

use super::{export::ExportCommand, import::ImportCommand};

const SUBSCRIBE_WORKFLOW_TIMEOUT: Duration = Duration::from_mins(2);
const SUBSCRIBE_RESULT_ENTRIES: i64 = 5;

/// Manage feeds
#[derive(Args, Debug)]
pub struct FeedCommand {
    #[command(subcommand)]
    command: FeedSubcommand,
}

#[derive(Subcommand, Debug)]
enum FeedSubcommand {
    Import(ImportCommand),
    Export(ExportCommand),
    Subscribe(SubscribeCommand),
    Unsubscribe(UnsubscribeCommand),
}

impl FeedCommand {
    pub async fn run(self, config: ConfigResolver) -> ExitCode {
        match self.command {
            FeedSubcommand::Import(import) => import.run(config).await,
            FeedSubcommand::Export(export) => export.run(config).await,
            FeedSubcommand::Subscribe(subscribe) => subscribe.run(config).await,
            FeedSubcommand::Unsubscribe(unsubscribe) => unsubscribe.run(),
        }
    }
}

/// Subscribe to a feed
#[derive(Args, Debug)]
struct SubscribeCommand {
    /// Feed URL
    #[arg(long)]
    url: String,
    /// Feed category
    #[arg(long)]
    category: Option<String>,
    /// Feed requirement: must, should, or may
    #[arg(long)]
    requirement: Option<String>,
}

impl SubscribeCommand {
    async fn run(self, config: ConfigResolver) -> ExitCode {
        match self.subscribe(config).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => CommandFailure::report(err),
        }
    }

    async fn subscribe(self, config: ConfigResolver) -> anyhow::Result<()> {
        let input = self.input()?;
        let url = input.url.clone();
        let cx = PortContext::new(&config).await?;
        let result = async {
            let events = cx.client.subscribe_feed_events().await?;
            let response = cx.client.subscribe_feed(input).await?;
            SubscribeWorkflow::new(url, response.request_id)
                .run(&cx.client, events, io::stdout())
                .await
        }
        .await;

        cx.finish(result).await
    }

    fn input(self) -> anyhow::Result<SubscribeFeedInput> {
        let Self {
            url,
            category,
            requirement,
        } = self;
        let url = FeedUrl::parse(&url)?;
        let category: Option<Category<'static>> = category.map(Category::new).transpose()?;
        let requirement = requirement
            .as_deref()
            .map(|value| value.parse::<Requirement>().map_err(anyhow::Error::msg))
            .transpose()?;

        Ok(SubscribeFeedInput {
            url,
            requirement,
            category,
            crawl_policy: None,
        })
    }
}

struct SubscribeWorkflow {
    url: FeedUrl,
    request_id: String,
    subscription_confirmed: bool,
    feed_observed: bool,
    fetch_started: bool,
}

impl SubscribeWorkflow {
    fn new(url: FeedUrl, request_id: String) -> Self {
        Self {
            url,
            request_id,
            subscription_confirmed: false,
            feed_observed: false,
            fetch_started: false,
        }
    }

    async fn run(
        mut self,
        client: &Client,
        mut events: mpsc::UnboundedReceiver<FeedEvent>,
        mut out: impl io::Write,
    ) -> anyhow::Result<()> {
        let deadline = Instant::now() + SUBSCRIBE_WORKFLOW_TIMEOUT;

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                anyhow::bail!("timed out while waiting for feed subscription progress");
            }

            let event = tokio::time::timeout(remaining, events.recv()).await?;
            let Some(event) = event else {
                anyhow::bail!("feed event stream closed while subscribing to {}", self.url);
            };

            if self.apply_event(&event, &mut out)?
                && self
                    .try_print_result(client, &mut out, ResultReadiness::RequireEntry)
                    .await?
            {
                return Ok(());
            }

            if self.feed_observed
                && self
                    .try_print_result(client, &mut out, ResultReadiness::AllowEmpty)
                    .await?
            {
                return Ok(());
            }
        }
    }

    fn apply_event(&mut self, event: &FeedEvent, out: &mut impl io::Write) -> anyhow::Result<bool> {
        match event {
            FeedEvent::FeedSubscribed(event) if event.request_id == self.request_id => {
                self.subscription_confirmed = true;
                Ok(true)
            }
            FeedEvent::SubscriptionChanged(event) if event.request_id == self.request_id => {
                self.subscription_confirmed = true;
                Ok(true)
            }
            FeedEvent::FeedSubscribeRejected(event) if event.request_id == self.request_id => {
                anyhow::bail!("failed to subscribe {}: {}", event.url, event.reason)
            }
            FeedEvent::CrawlJobEnqueued(event) if event.url == self.url => {
                writeln!(out, "queued {} ...", self.url)?;
                Ok(true)
            }
            FeedEvent::CrawlJobStarted(event) if event.url == self.url => {
                if !self.fetch_started {
                    writeln!(out, "fetching {} ...", self.url)?;
                    self.fetch_started = true;
                }
                Ok(true)
            }
            FeedEvent::CrawlJobFinished(event) if event.url == self.url => {
                if let Some(error) = &event.error {
                    writeln!(out, "{} subscribed, but first fetch failed.", self.url)?;
                    if let Some(status) = event.http_status {
                        writeln!(out, "http_status: {status}")?;
                    }
                    writeln!(out, "reason: {error}")?;
                    return Err(anyhow::anyhow!("first fetch failed for {}", self.url));
                }
                Ok(true)
            }
            FeedEvent::FeedDiscovered(event) if event.url == self.url => {
                self.feed_observed = true;
                Ok(true)
            }
            FeedEvent::FeedChanged(event) if event.url == self.url => {
                self.feed_observed = true;
                Ok(true)
            }
            FeedEvent::EntryDiscovered(event) if event.url == self.url => Ok(true),
            FeedEvent::EntryChanged(event) if event.url == self.url => Ok(true),
            FeedEvent::TimelineChanged(event)
                if event
                    .affected_feeds
                    .as_ref()
                    .is_some_and(|feeds| feeds.contains(&self.url)) =>
            {
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn try_print_result(
        &self,
        client: &Client,
        out: &mut impl io::Write,
        readiness: ResultReadiness,
    ) -> anyhow::Result<bool> {
        if !self.subscription_confirmed {
            return Ok(false);
        }

        let payload = client
            .fetch_feed_entries(self.url.clone(), None, SUBSCRIBE_RESULT_ENTRIES)
            .await?;
        if payload.entries.is_empty() && readiness == ResultReadiness::RequireEntry {
            return Ok(false);
        }

        writeln!(out, "{} subscribed.", self.url)?;
        if let Some(title) = payload
            .entries
            .iter()
            .find_map(|entry| entry.feed.title.as_deref())
        {
            writeln!(out, "title: {title}")?;
        }
        if payload.entries.is_empty() {
            writeln!(out, "entries: 0")?;
            return Ok(true);
        }

        writeln!(out, "entries:")?;
        for entry in payload.entries {
            let title = entry.title.as_deref().unwrap_or("(untitled)");
            writeln!(out, "- {title}")?;
        }

        Ok(true)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResultReadiness {
    RequireEntry,
    AllowEmpty,
}

/// Unsubscribe from a feed
#[derive(Args, Debug)]
struct UnsubscribeCommand {
    /// Feed URL
    #[arg(long)]
    url: String,
}

impl UnsubscribeCommand {
    fn run(self) -> ExitCode {
        let Self { url } = self;
        let _ = url;
        not_yet_implemented()
    }
}

fn not_yet_implemented() -> ExitCode {
    eprintln!("not_yet_implemented");
    ExitCode::from(1)
}

#[cfg(test)]
mod tests {
    use synd_client::payload::{
        CrawlJobStartedEvent, FeedSubscribeRejectedEvent, FeedSubscribedEvent,
    };

    use super::*;

    #[test]
    fn subscribe_progress_hides_request_id_and_prints_fetch_start() -> anyhow::Result<()> {
        let url = feed_url();
        let mut workflow = SubscribeWorkflow::new(url.clone(), "request-1".to_owned());
        let mut out = Vec::new();

        assert!(workflow.apply_event(
            &FeedEvent::FeedSubscribed(FeedSubscribedEvent {
                request_id: "request-1".to_owned(),
                url: url.clone(),
            }),
            &mut out,
        )?);
        assert!(workflow.subscription_confirmed);
        assert!(out.is_empty());

        assert!(workflow.apply_event(
            &FeedEvent::CrawlJobStarted(CrawlJobStartedEvent { url: url.clone() }),
            &mut out,
        )?);

        let output = String::from_utf8(out)?;
        assert_eq!(output, format!("fetching {url} ...\n"));
        assert!(!output.contains("request-1"));
        Ok(())
    }

    #[test]
    fn subscribe_rejected_error_hides_request_id() {
        let url = feed_url();
        let mut workflow = SubscribeWorkflow::new(url.clone(), "request-1".to_owned());
        let mut out = Vec::new();

        let err = workflow
            .apply_event(
                &FeedEvent::FeedSubscribeRejected(FeedSubscribeRejectedEvent {
                    request_id: "request-1".to_owned(),
                    url,
                    reason: "denied".to_owned(),
                }),
                &mut out,
            )
            .unwrap_err();

        let message = err.to_string();
        assert!(message.contains("failed to subscribe"));
        assert!(message.contains("denied"));
        assert!(!message.contains("request-1"));
        assert!(out.is_empty());
    }

    fn feed_url() -> FeedUrl {
        FeedUrl::parse("https://example.com/feed.xml").unwrap()
    }
}
