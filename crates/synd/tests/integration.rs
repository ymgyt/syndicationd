#[cfg(feature = "integration")]
mod test {
    use std::time::Duration;

    use synd_feed::types::{Category, FeedUrl, Requirement};
    use synd_term::{auth::Credential, key, test_support::screen::Screen};

    mod helper;
    use crate::test::helper::{SubscriptionSeed, TestCase};

    mod auth_wiring {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn gh_device_flow() -> anyhow::Result<()> {
            helper::init_tracing();

            let mut application = TestCase::default().init_app().await?;
            let tx = application.input();

            application.wait_until_jobs_completed().await;
            tx.send(key!(enter));
            tokio::time::timeout(
                Duration::from_secs(10),
                application.run_until(|buffer| Screen::new(buffer).contains_text("UC123456")),
            )
            .await??;

            let screen = Screen::new(application.buffer());
            assert!(screen.contains_text("Login"));
            assert!(screen.contains_text("https://syndicationd.ymgyt.io/test"));
            assert!(screen.contains_text("UC123456"));

            Ok(())
        }
    }

    mod feed_wiring {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn seeded_subscriptions() -> anyhow::Result<()> {
            let feed_url: FeedUrl = "https://example.com/feed.xml".try_into().unwrap();
            let test_case = TestCase {
                terminal_col_row: (120, 30),
                subscriptions: vec![SubscriptionSeed::interval(
                    feed_url.clone(),
                    Some(Requirement::Must),
                    Some(Category::new("rust").unwrap()),
                    Duration::from_hours(1),
                )],
                ..Default::default()
            }
            .already_logined();

            let mut application = test_case.init_app().await?;
            let tx = application.input();
            application.wait_until_jobs_completed().await;

            tx.send(key!(tab));
            application.wait_until_jobs_completed().await;

            let screen = Screen::new(application.buffer());
            assert!(screen.contains_text("Syndicationd"));
            assert!(screen.contains_text("Feeds"));
            assert!(screen.contains_text(feed_url.as_ref()));
            assert!(screen.contains_text("MUST"));
            assert!(screen.contains_text("rust"));

            Ok(())
        }
    }

    mod auth_errors {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn unauthorized() -> anyhow::Result<()> {
            let test_case = TestCase {
                terminal_col_row: (120, 30),
                ..Default::default()
            }
            .with_credential(Credential::Gh {
                access_token: synd_test::GITHUB_INVALID_TOKEN.to_owned(),
            });

            let mut application = test_case.init_app().await?;
            application.wait_until_jobs_completed().await;

            let screen = Screen::new(application.buffer());
            assert!(screen.contains_text("Login"));
            assert!(screen.contains_text("unauthorized. local feed API session is invalid"));

            Ok(())
        }
    }
}
