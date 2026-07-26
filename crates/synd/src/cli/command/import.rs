use std::{
    io,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

use clap::Args;
use either::Either;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synd_client::{
    Client, SyndApiError,
    payload::{SubscribeFeedInput, SubscribeFeedPayload},
};
use synd_term::{types::ExportedFeed, ui};

use crate::{
    cli::{command::CommandFailure, port::PortContext},
    config::ConfigResolver,
};

#[derive(Serialize, Deserialize, JsonSchema)]
struct Input {
    feeds: Vec<ExportedFeed>,
}

/// Import subscribed feeds
#[derive(Args, Debug)]
pub struct ImportCommand {
    /// Print json schema for import data
    #[arg(
        long,
        default_value_t = false,
        action = clap::ArgAction::SetTrue,
        visible_alias = "print-json-schema",
    )]
    print_schema: bool,
    /// Path to input file, '-' means stdin.
    #[arg()]
    input: Option<PathBuf>,
}

impl ImportCommand {
    pub async fn run(self, config: ConfigResolver) -> ExitCode {
        let err = if self.print_schema {
            Self::print_json_schema()
        } else {
            self.import(config).await
        };
        if let Err(err) = err {
            CommandFailure::report(err)
        } else {
            ExitCode::SUCCESS
        }
    }

    fn print_json_schema() -> anyhow::Result<()> {
        let schema = schemars::schema_for!(Input);
        serde_json::to_writer_pretty(std::io::stdout(), &schema).map_err(anyhow::Error::from)
    }

    async fn import(self, config: ConfigResolver) -> anyhow::Result<()> {
        let Self {
            print_schema: _,
            input,
        } = self;

        let input = match input {
            Some(input) => Self::read_input(input.as_path())?,
            None => {
                anyhow::bail!("input file path required")
            }
        };
        let cx = PortContext::new(&config).await?;
        let import = Import {
            client: &cx.client,
            input,
            out: io::stdout(),
            interval: Duration::from_millis(500),
        };

        let result = import.import().await;

        cx.finish(result).await
    }

    fn read_input(path: &Path) -> anyhow::Result<Input> {
        let src = if path == Path::new("-") {
            Either::Left(std::io::stdin())
        } else {
            Either::Right(std::fs::File::open(path)?)
        };

        serde_json::from_reader(src).map_err(anyhow::Error::from)
    }
}

#[cfg_attr(test, mockall::automock)]
trait SubscribeFeed {
    async fn subscribe_feed(
        &self,
        input: SubscribeFeedInput,
    ) -> Result<SubscribeFeedPayload, SyndApiError>;
}

impl SubscribeFeed for Client {
    async fn subscribe_feed(
        &self,
        input: SubscribeFeedInput,
    ) -> Result<SubscribeFeedPayload, SyndApiError> {
        Client::subscribe_feed(self, input).await
    }
}

impl SubscribeFeed for &Client {
    async fn subscribe_feed(
        &self,
        input: SubscribeFeedInput,
    ) -> Result<SubscribeFeedPayload, SyndApiError> {
        Client::subscribe_feed(self, input).await
    }
}

/// Represents import process
struct Import<Client, Out> {
    client: Client,
    input: Input,
    out: Out,
    interval: Duration,
}

impl<Client, Out> Import<Client, Out>
where
    Client: SubscribeFeed,
    Out: io::Write,
{
    async fn import(self) -> anyhow::Result<()> {
        let Import {
            client,
            input,
            mut out,
            interval,
        } = self;

        let max_category_width = input
            .feeds
            .iter()
            .map(|f| {
                f.category
                    .as_ref()
                    .map_or(0, |c| c.as_str().chars().count())
            })
            .max()
            .unwrap_or(0);

        let feeds_count = input.feeds.len();
        let mut ok: usize = 0;
        let mut interval = tokio::time::interval(interval);

        for feed in input.feeds {
            interval.tick().await;
            let url = feed.url.clone();
            let requirement = feed.requirement.unwrap_or(ui::DEFAULT_REQUIREMENT);
            let category = feed
                .category
                .clone()
                .unwrap_or_else(|| ui::default_category().clone());
            let input = match SubscribeFeedInput::try_from(feed) {
                Ok(input) => input,
                Err(error) => {
                    writeln!(&mut out, "ERROR {url} {error}")?;
                    continue;
                }
            };
            match client.subscribe_feed(input).await {
                Ok(_) => {
                    writeln!(
                        &mut out,
                        "OK    {requirement:<6} {category:<max_category_width$} {url}",
                    )?;
                    ok = ok.saturating_add(1);
                }
                Err(err) => {
                    writeln!(&mut out, "ERROR {url} {err}")?;
                }
            }
        }

        writeln!(&mut out, "{ok}/{feeds_count} feeds successfully subscribed")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use synd_feed::types::{Category, FeedUrl, Requirement};

    #[tokio::test]
    async fn import_feeds_reports_success_and_failure() {
        let url_ok1: FeedUrl = "https://ok1.ymgyt.io/feed.xml".try_into().unwrap();
        let url_ok2: FeedUrl = "https://ok2.ymgyt.io/feed.xml".try_into().unwrap();
        let url_unavailable: FeedUrl = "https://err_unavailable.ymgyt.io/feed.xml"
            .try_into()
            .unwrap();
        let cat_rust = Category::new("rust").unwrap();
        let cat_long = Category::new("longcategory").unwrap();

        let input = Input {
            feeds: vec![
                ExportedFeed {
                    title: Some(String::from("ok1")),
                    url: url_ok1.clone(),
                    requirement: Some(Requirement::Must),
                    category: Some(cat_rust.clone()),
                    crawl_policy: None,
                },
                ExportedFeed {
                    title: Some(String::from("err unuvailable")),
                    url: url_unavailable.clone(),
                    requirement: Some(Requirement::Must),
                    category: Some(cat_rust.clone()),
                    crawl_policy: None,
                },
                ExportedFeed {
                    title: Some(String::from("ok2")),
                    url: url_ok2.clone(),
                    requirement: Some(Requirement::Should),
                    category: Some(cat_long.clone()),
                    crawl_policy: None,
                },
            ],
        };

        let interval = Duration::from_millis(100);
        let mut prev = None;
        let mut client = MockSubscribeFeed::new();

        client.expect_subscribe_feed().returning(move |input| {
            let now = Instant::now();
            if let Some(prev) = prev {
                assert!(
                    // Dut to insability in the CI execution
                    // the interval assertion has been relaxed
                    now.duration_since(prev)
                        >= interval
                            .checked_sub(Duration::from_millis(50))
                            .unwrap_or_default(),
                    "the interval between requests is too short"
                );
            }
            prev = Some(now);
            match input.url.as_str() {
                "https://ok1.ymgyt.io/feed.xml" => Ok(SubscribeFeedPayload {
                    status: synd_client::payload::ResponseStatus {
                        code: synd_client::payload::ResponseCode::Ok,
                    },
                    url: url_ok1.clone(),
                    disposition: synd_client::payload::SubscribeDisposition::Subscribed,
                }),
                "https://ok2.ymgyt.io/feed.xml" => Ok(SubscribeFeedPayload {
                    status: synd_client::payload::ResponseStatus {
                        code: synd_client::payload::ResponseCode::Ok,
                    },
                    url: url_ok2.clone(),
                    disposition: synd_client::payload::SubscribeDisposition::Subscribed,
                }),
                "https://err_unavailable.ymgyt.io/feed.xml" => {
                    Err(SyndApiError::UnexpectedResponse {
                        context: "server returned 500 error",
                    })
                }
                _ => panic!(),
            }
        });

        let mut out = Vec::new();

        let import = Import {
            client,
            input,
            out: &mut out,
            interval,
        };

        import.import().await.unwrap();

        let buf = String::from_utf8_lossy(out.as_slice());
        insta::with_settings!({
            description => "import command output"
        }, {
            insta::assert_snapshot!("import_feeds_reports_success_and_failure",buf);
        });
    }
}
