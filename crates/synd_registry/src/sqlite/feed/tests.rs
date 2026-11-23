use synd_feed::types::{Link, Text};

use crate::sqlite::test_support::*;

#[tokio::test]
async fn article_urls_are_cleaned_after_identification_and_before_deduplication()
-> anyhow::Result<()> {
    let url = feed_url("article-urls");
    let body = br#"<rss version="2.0"><channel><title>News</title>
        <item><link>https://example.com/post?id=42&amp;utm_source=a#comments</link></item>
        <item><link>https://example.com/post?id=42&amp;utm_source=b#comments</link></item>
        <item><guid>stable</guid><link>https://example.com/post?utm_source=a</link></item>
        <item><guid>stable</guid><link>https://example.com/post?utm_source=b</link></item>
    </channel></rss>"#;
    let db = migrated_db().await?;
    let crawl = record_fetched_crawl(&db, &url, body.to_vec(), 0).await?;
    project_feed(&db, crawl).await?;

    let mut tx = db.begin().await?;
    let feeds = tx.load_feeds(std::slice::from_ref(&url)).await?;
    tx.commit().await?;
    let feed = &feeds[&url];
    let mut articles: Vec<_> = feed
        .entries()
        .map(|entry| entry.website_url(feed.meta().r#type()).unwrap())
        .collect();
    articles.sort_unstable();
    assert_eq!(
        articles,
        [
            "https://example.com/post",
            "https://example.com/post?id=42#comments",
            "https://example.com/post?id=42#comments",
        ]
    );
    Ok(())
}

#[tokio::test]
async fn removes_utm_parameters_from_each_unsigned_entry_link() -> anyhow::Result<()> {
    let url = feed_url("entry-links");
    let body = br#"<feed xmlns="http://www.w3.org/2005/Atom"><title>News</title>
        <link rel="alternate" href="https://example.com/?utm_source=feed"/>
        <entry><id>urn:example:1</id><title>Post</title>
            <link rel="alternate" href="https://example.com/post?utm_source=rss"/>
            <link rel="related" href="https://example.com/related?utm_source=rss&amp;id=2"/>
            <link rel="related" href="https://example.com/file?utm_source=rss&amp;X-Amz-Signature=abc"/>
        </entry></feed>"#;
    let db = migrated_db().await?;
    let crawl = record_fetched_crawl(&db, &url, body.to_vec(), 0).await?;
    project_feed(&db, crawl).await?;

    let mut tx = db.begin().await?;
    let feeds = tx.load_feeds(std::slice::from_ref(&url)).await?;
    tx.commit().await?;
    let feed = &feeds[&url];
    assert_eq!(
        feed.meta().website_url(),
        Some("https://example.com/?utm_source=feed")
    );
    let entry = feed.entries().next().unwrap();
    let hrefs: Vec<_> = entry.links().iter().map(Link::href).collect();
    assert_eq!(
        hrefs,
        [
            "https://example.com/post",
            "https://example.com/related?id=2",
            "https://example.com/file?utm_source=rss&X-Amz-Signature=abc",
        ]
    );
    Ok(())
}

fn rss_with_category(category_element: &str) -> Vec<u8> {
    format!(
        "<rss version='2.0'><channel><title>News</title><item><guid>news</guid>\
         <title>New Regions</title>{category_element}</item></channel></rss>"
    )
    .into_bytes()
}

#[tokio::test]
async fn rss_category_terms_round_trip_and_only_set_changes_emit_events() -> anyhow::Result<()> {
    use crate::event::EntryChangedEvent;

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
    use crate::event::EntryChangedEvent;
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
