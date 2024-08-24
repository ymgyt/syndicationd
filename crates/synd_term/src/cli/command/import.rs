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
use url::Url;

use crate::{
    cli::port::PortContext,
    client::{
        mutation::subscribe_feed::SubscribeFeedInput, Client, SubscribeFeedError, SyndApiError,
    },
    config,
    types::{self, ExportedFeed},
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
    /// Cache directory
    #[arg(
        long,
        default_value = config::cache::dir().to_path_buf().into_os_string(),
    )]
    cache_dir: PathBuf,
    /// Path to input file, '-' means stdin.
    #[arg()]
    input: PathBuf,
}

impl ImportCommand {
    pub async fn run(self, endpoint: Url) -> ExitCode {
        let err = if self.print_schema {
            Self::print_json_schema()
        } else {
            self.import(endpoint).await
        };
        if let Err(err) = err {
            tracing::error!("{err:?}");
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        }
    }

    fn print_json_schema() -> anyhow::Result<()> {
        let schema = schemars::schema_for!(Input);
        serde_json::to_writer_pretty(std::io::stdout(), &schema).map_err(anyhow::Error::from)
    }

    async fn import(self, endpoint: Url) -> anyhow::Result<()> {
        let cx = PortContext::new(endpoint, self.cache_dir).await?;
        let import = Import {
            client: cx.client,
            input: Self::read_input(self.input.as_path())?,
            out: io::stdout(),
            interval: Duration::from_millis(500),
        };

        import.import().await
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
    async fn subscribe_feed(&self, input: SubscribeFeedInput) -> Result<types::Feed, SyndApiError>;
}

impl SubscribeFeed for Client {
    async fn subscribe_feed(&self, input: SubscribeFeedInput) -> Result<types::Feed, SyndApiError> {
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
            match client.subscribe_feed(SubscribeFeedInput::from(feed)).await {
                Ok(imported) => {
                    writeln!(
                        &mut out,
                        "OK    {req:<6} {category:<cat_width$} {url}",
                        req = imported.requirement(),
                        category = imported.category(),
                        cat_width = max_category_width,
                        url = imported.url,
                    )?;
                    ok = ok.saturating_add(1);
                }
                Err(SyndApiError::SubscribeFeed(SubscribeFeedError::FeedUnavailable {
                    feed_url,
                    message,
                })) => {
                    writeln!(&mut out, "ERROR {feed_url} {message}",)?;
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
    use fake::{Fake as _, Faker};
    use synd_feed::types::{Category, FeedUrl, Requirement};

    #[tokio::test]
    async fn usecase() {
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
                },
                ExportedFeed {
                    title: Some(String::from("err unuvailable")),
                    url: url_unavailable.clone(),
                    requirement: Some(Requirement::Must),
                    category: Some(cat_rust.clone()),
                },
                ExportedFeed {
                    title: Some(String::from("ok2")),
                    url: url_ok2.clone(),
                    requirement: Some(Requirement::Should),
                    category: Some(cat_long.clone()),
                },
            ],
        };

        let base_feed: types::Feed = Faker.fake();
        let interval = Duration::from_millis(10);
        let mut prev = None;
        let mut client = MockSubscribeFeed::new();

        client.expect_subscribe_feed().returning(move |input| {
            let now = Instant::now();
            if let Some(prev) = prev {
                assert!(
                    now.duration_since(prev) > interval,
                    "the interval between requests is too short"
                );
            }
            prev = Some(now);

            match input.url.as_str() {
                "https://ok1.ymgyt.io/feed.xml" => Ok(base_feed
                    .clone()
                    .with_url(url_ok1.clone())
                    .with_requirement(Requirement::Must)
                    .with_category(cat_rust.clone())),
                "https://ok2.ymgyt.io/feed.xml" => Ok(base_feed
                    .clone()
                    .with_url(url_ok2.clone())
                    .with_requirement(Requirement::Should)
                    .with_category(cat_long.clone())),
                "https://err_unavailable.ymgyt.io/feed.xml" => Err(SyndApiError::SubscribeFeed(
                    SubscribeFeedError::FeedUnavailable {
                        feed_url: url_unavailable.clone(),
                        message: "server return 500 error".into(),
                    },
                )),
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
        // insta::with_settings!({
        //     description => "import command output"
        // }, {
        //     insta::assert_snapshot!("import_usecase",buf);
        // });
        println!("{buf}");
    }
}
