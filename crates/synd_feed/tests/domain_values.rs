use synd_feed::types::{Content, Entry, EntryId, FeedType, Link, Person, Text};

#[test]
fn domain_values_are_publicly_constructible() {
    let title = Text::builder()
        .content("Entry title".to_owned())
        .content_type("text/plain".to_owned())
        .build();
    let author = Person::builder()
        .name("Author".to_owned())
        .uri("https://example.com/author".to_owned())
        .email("author@example.com".to_owned())
        .build();
    let link = Link::builder()
        .href("https://example.com/entry".to_owned())
        .rel("alternate".to_owned())
        .media_type("text/html".to_owned())
        .href_lang("en".to_owned())
        .title("Entry".to_owned())
        .length(10)
        .build();
    let content = Content::builder()
        .body("Entry body".to_owned())
        .content_type("text/plain".to_owned())
        .length(10)
        .src(link.clone())
        .build();
    let entry = Entry::builder()
        .id(EntryId::parse(format!("synd:entry:v1:{}", "a".repeat(64))).unwrap())
        .title(title.clone())
        .authors(vec![author.clone()])
        .content(content.clone())
        .links(vec![link.clone()])
        .summary(title.clone())
        .build();

    assert_eq!(title.content(), "Entry title");
    assert_eq!(author.name(), "Author");
    assert_eq!(link.href(), "https://example.com/entry");

    assert_eq!(entry.website_url(FeedType::Atom), Some(link.href()));

    let json = serde_json::to_string(&entry).unwrap();
    let decoded = serde_json::from_str::<Entry>(&json).unwrap();
    assert_eq!(decoded, entry);
}
