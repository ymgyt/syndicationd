use synd_feed::types::Text;

use crate::sqlite::feed_registry::test_support::*;

fn rss_with_category(category_element: &str) -> Vec<u8> {
    format!(
        "<rss version='2.0'><channel><title>News</title><item><guid>news</guid>\
         <title>New Regions</title>{category_element}</item></channel></rss>"
    )
    .into_bytes()
}

#[tokio::test]
async fn rss_category_terms_round_trip_and_only_set_changes_emit_events() -> anyhow::Result<()> {
    use synd_registry::event::EntryChangedEvent;

    let url = feed_url("news");
    let db = migrated_db().await?;

    let observations = [
        (
            "auth,topic:systems/kernel",
            vec![EntryDiscoveredEvent::TYPE],
            vec!["auth", "topic:systems/kernel"],
        ),
        (
            "topic:systems/kernel, auth,, auth",
            vec![],
            vec!["auth", "topic:systems/kernel"],
        ),
        (
            "topic:systems/kernel",
            vec![EntryChangedEvent::TYPE],
            vec!["topic:systems/kernel"],
        ),
        ("", vec![EntryChangedEvent::TYPE], vec![]),
    ];
    for (seq, (category, events, terms)) in (0..).zip(observations) {
        let body = rss_with_category(&format!("<category>{category}</category>"));
        let crawl = record_fetched_crawl(&db, &url, body, seq).await?;
        assert_eq!(project_feed(&db, crawl).await?.types(), events);
        let mut tx = db.begin().await?;
        let feeds = tx.load_feeds(std::slice::from_ref(&url)).await?;
        tx.commit().await?;
        let entry = feeds[&url].entries().next().unwrap();
        let categories = entry.source_categories()?;
        assert_eq!(categories.is_none(), terms.is_empty());
        let stored: Vec<_> = categories
            .into_iter()
            .flatten()
            .map(|category| category.term)
            .collect();
        assert_eq!(stored, terms);
    }
    Ok(())
}

#[tokio::test]
async fn only_rss_category_terms_are_comma_delimited() -> anyhow::Result<()> {
    let url = feed_url("categories");
    let db = migrated_db().await?;
    let rss = rss_with_category("<category domain='topics'>a,b</category>");
    let atom = br#"<feed xmlns="http://www.w3.org/2005/Atom">
        <title>News</title><entry><id>news</id><title>New Regions</title>
        <category term="a,b" scheme="topics" label="A and B"/></entry></feed>"#;
    let cases = [
        (
            rss,
            serde_json::json!([{"term":"a","scheme":"topics"}, {"term":"b","scheme":"topics"}]),
        ),
        (
            atom.to_vec(),
            serde_json::json!([{"term":"a,b","scheme":"topics","label":"A and B"}]),
        ),
    ];
    for (seq, (body, expected)) in (0..).zip(cases) {
        let crawl = record_fetched_crawl(&db, &url, body, seq).await?;
        project_feed(&db, crawl).await?;
        let mut tx = db.begin().await?;
        let feeds = tx.load_feeds(std::slice::from_ref(&url)).await?;
        tx.commit().await?;
        let entry = feeds[&url].entries().next().unwrap();
        assert_eq!(entry.labels()["source.categories"], expected);
    }
    Ok(())
}

#[tokio::test]
async fn feed_projection_stores_a_complete_current_feed() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let feed_url = feed_url("feed-projection");
    let crawl = record_fetched_crawl(
        &db,
        &feed_url,
        rss_body_with_entry("feed title", "entry title", "entry-1"),
        0,
    )
    .await?;

    let recorded = project_feed(&db, crawl).await?;

    assert_eq!(recorded.types(), &[EntryDiscoveredEvent::TYPE]);
    let mut tx = db.begin().await?;
    let mut feeds = tx.load_feeds(std::slice::from_ref(&feed_url)).await?;
    tx.commit().await?;
    let feed = feeds.remove(&feed_url).unwrap();
    assert_eq!(feed.meta().title().map(Text::content), Some("feed title"));
    assert_eq!(
        feed.entries()
            .next()
            .and_then(|entry| entry.title())
            .map(Text::content),
        Some("entry title")
    );
    Ok(())
}

#[tokio::test]
async fn feed_projection_replaces_membership_without_deleting_catalog_history() -> anyhow::Result<()>
{
    let db = migrated_db().await?;
    let feed_url = feed_url("feed-membership");
    let first = record_fetched_crawl(
        &db,
        &feed_url,
        rss_body_with_entry("feed", "old entry", "entry-1"),
        0,
    )
    .await?;
    project_feed(&db, first).await?;
    let second = record_fetched_crawl(
        &db,
        &feed_url,
        rss_body_with_entry("feed", "current entry", "entry-2"),
        1,
    )
    .await?;

    project_feed(&db, second).await?;

    let mut tx = db.begin().await?;
    let mut feeds = tx.load_feeds(std::slice::from_ref(&feed_url)).await?;
    let catalog_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM entry")
        .fetch_one(&mut *tx.tx)
        .await?;
    tx.commit().await?;
    let feed = feeds.remove(&feed_url).unwrap();
    let titles = feed
        .entries()
        .filter_map(|entry| entry.title().map(Text::content))
        .collect::<Vec<_>>();
    assert_eq!(titles, ["current entry"]);
    assert_eq!(catalog_count, 2);
    Ok(())
}

fn rss_with_items(items: &[(&str, &str)]) -> Vec<u8> {
    use std::fmt::Write;
    let mut body = String::from(
        r#"<rss version="2.0"><channel><title>feed</title><link>https://example.com/</link><description>feed</description>"#,
    );
    for (id, title) in items {
        write!(body, "<item><guid>{id}</guid><title>{title}</title></item>").unwrap();
    }
    body.push_str("</channel></rss>");
    body.into_bytes()
}

#[tokio::test]
async fn conflicting_ids_preserve_only_existing_membership_while_valid_entries_are_applied()
-> anyhow::Result<()> {
    use synd_registry::event::EntryChangedEvent;
    let db = migrated_db().await?;
    let url = feed_url("partial-conflict");
    let initial = rss_with_items(&[("b", "keep"), ("d", "old"), ("e", "expired")]);
    project_feed(&db, record_fetched_crawl(&db, &url, initial, 0).await?).await?;
    let mixed = rss_with_items(&[
        ("b", "conflict one"),
        ("b", "conflict two"),
        ("b", "conflict one"),
        ("x", "new conflict one"),
        ("x", "new conflict two"),
        ("d", "updated"),
        ("c", "new"),
        ("c", "new"),
    ]);
    let event = record_fetched_crawl(&db, &url, mixed, 1).await?;
    let recorded = project_feed(&db, event.clone()).await?;
    assert_eq!(
        recorded.types(),
        &[EntryChangedEvent::TYPE, EntryDiscoveredEvent::TYPE]
    );
    assert_eq!(project_feed(&db, event).await?.types(), []);

    let mut tx = db.begin().await?;
    let feeds = tx.load_feeds(std::slice::from_ref(&url)).await?;
    let mut titles: Vec<_> = feeds[&url]
        .entries()
        .map(|entry| entry.title().unwrap().content())
        .collect();
    titles.sort_unstable();
    assert_eq!(titles, ["keep", "new", "updated"]);
    tx.commit().await?;

    // A later unambiguous observation can update the previously skipped entry.
    let recovered = rss_with_items(&[("b", "resolved")]);
    let recorded = project_feed(&db, record_fetched_crawl(&db, &url, recovered, 2).await?).await?;
    assert_eq!(recorded.types(), &[EntryChangedEvent::TYPE]);
    Ok(())
}
