#![cfg(unix)]

use std::{path::Path, process::Stdio, time::Duration};

use anyhow::{Context as _, ensure};
use serde_json::{Value, json};
use synd_client::{
    Client,
    payload::{CreateTimelineInput, TimelineEntriesRequest, TimelineId},
};
use synd_runtime::{Runtime, RuntimeConfig, RuntimeDatabase};
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader, Lines},
    net::TcpListener,
    process::{ChildStdin, ChildStdout, Command},
    time::timeout,
};

#[tokio::test]
async fn stdio_recovers_after_daemon_shutdown_and_closes_session() -> anyhow::Result<()> {
    timeout(Duration::from_secs(30), test_stdio()).await?
}

async fn test_stdio() -> anyhow::Result<()> {
    let dir = synd_test::temp_dir();
    let dir = dir.path();
    std::fs::write(dir.join("synd.toml"), "")?;

    let mut child = synd(dir)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut peer = McpPeer {
        stdin: child.stdin.take().unwrap(),
        stdout: BufReader::new(child.stdout.take().unwrap()).lines(),
    };

    let init = peer
        .request(
            "initialize",
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "synd-test", "version": "0"},
            }),
        )
        .await?;
    assert_eq!(init["protocolVersion"], "2025-11-25");
    assert!(init["capabilities"]["tools"].is_object());
    peer.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        .await?;
    assert_eq!(active_sessions(dir).await?, 1);

    let listed = peer.request("tools/list", json!({})).await?;
    let definitions = listed["tools"].as_array().unwrap();
    let tool = |name: &str| {
        definitions
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap()
    };
    let fetch = tool("fetch_entry_body");
    assert_eq!(fetch["inputSchema"]["required"], json!(["url"]));
    assert_eq!(
        fetch["outputSchema"]["properties"]["body"]["type"],
        "string"
    );
    assert_eq!(fetch["annotations"]["openWorldHint"], true);
    let timeline_entries = tool("list_timeline_entries");
    let params = &timeline_entries["inputSchema"]["properties"];
    assert!(params["maxUnicodeWidth"].is_object());
    assert_eq!(params["since"]["format"], "date-time");
    assert_eq!(params["until"]["format"], "date-time");
    let definitions_by_type = &timeline_entries["outputSchema"]["$defs"];
    let fields = &definitions_by_type["TimelineEntry"]["properties"];
    assert_eq!(fields["id"]["type"], "string");
    assert_eq!(fields["feed"]["$ref"], "#/$defs/Feed");
    let feed = &definitions_by_type["Feed"]["properties"];
    assert_eq!(feed["title"]["type"], json!(["string", "null"]));
    assert_eq!(feed["url"]["$ref"], "#/$defs/FeedUrl");
    assert_eq!(definitions_by_type["FeedUrl"]["format"], "uri");
    assert_eq!(fields["published"]["format"], "date-time");
    assert_eq!(fields["orderTime"]["format"], "date-time");
    let mut tools: Vec<&str> = definitions
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    tools.sort_unstable();
    assert_eq!(
        tools,
        [
            "fetch_entry_body",
            "list_subscriptions",
            "list_timeline_entries",
            "list_timelines",
            "subscribe_feed",
            "unsubscribe_feed"
        ]
    );

    check_fetch_entry_body(&mut peer).await?;

    let first = "http://127.0.0.1:1/first.xml";
    let second = "http://127.0.0.1:1/second.xml";
    for url in [first, second] {
        let subscribed = peer.tool("subscribe_feed", json!({"url": url})).await?;
        assert_eq!(subscribed, json!({"url": url, "disposition": "subscribed"}));
    }
    let page = peer.tool("list_subscriptions", json!({"first": 1})).await?;
    assert_eq!(page["items"], json!([{"url": first, "title": null}]));
    let next_cursor = page["next_cursor"]
        .as_str()
        .context("No continuation cursor")?;
    let page = peer
        .tool(
            "list_subscriptions",
            json!({"first": 1, "after": next_cursor}),
        )
        .await?;
    assert_eq!(
        page,
        json!({"items": [{"url": second, "title": null}], "next_cursor": null})
    );

    check_timeline_tools(&mut peer, dir).await?;

    shutdown_daemon(dir).await?;
    let removed = peer.tool("unsubscribe_feed", json!({"url": first})).await?;
    assert_eq!(
        removed,
        json!({"url": first, "disposition": "unsubscribed"})
    );
    assert_eq!(active_sessions(dir).await?, 1);

    drop(peer.stdin);
    assert!(child.wait().await?.success());
    assert!(peer.stdout.next_line().await?.is_none());
    assert_eq!(active_sessions(dir).await?, 0);
    shutdown_daemon(dir).await
}

async fn check_fetch_entry_body(peer: &mut McpPeer) -> anyhow::Result<()> {
    use axum::{Router, http::header, response::Redirect, routing::get};

    const PATCH_PAGE: &[u8] = b"<html><head><title>Patch</title></head><body><article>\
        <p>Read the <a href=\"notes\">notes by Ren\xe9</a> for this change.</p>\
        <pre><span>@@ -1,2 +1,2 @@</span>\n     context\n\
        <span>-\told</span>\n<span>+\tnew</span></pre></article></body></html>";

    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let publisher = Router::new()
        .route(
            "/entry",
            get(|| async { Redirect::temporary("/posts/patch") }),
        )
        .route(
            "/posts/patch",
            get(|| async {
                (
                    [(
                        header::CONTENT_TYPE,
                        "application/xhtml+xml; charset=iso-8859-1",
                    )],
                    PATCH_PAGE,
                )
            }),
        )
        .route("/plain", get(|| async { "plain text" }));
    let server = tokio::spawn(async move { axum::serve(listener, publisher).await });

    let result = peer
        .tool(
            "fetch_entry_body",
            json!({"url": format!("{origin}/entry")}),
        )
        .await?;
    assert_eq!(result["url"], format!("{origin}/posts/patch"));
    let body = result["body"].as_str().context("Missing Markdown body")?;
    assert!(
        body.contains(&format!("[notes by René]({origin}/posts/notes)")),
        "{body}"
    );
    assert!(
        body.contains("@@ -1,2 +1,2 @@\n     context\n-\told\n+\tnew"),
        "{body}"
    );

    let error = peer
        .tool(
            "fetch_entry_body",
            json!({"url": format!("{origin}/plain")}),
        )
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("expected an HTML article, received text/plain"),
        "{error}"
    );
    server.abort();
    Ok(())
}

async fn check_timeline_tools(peer: &mut McpPeer, dir: &Path) -> anyhow::Result<()> {
    let timelines = peer.tool("list_timelines", json!({})).await?;
    assert_eq!(timelines, json!({"items": []}));
    let error = peer
        .tool("list_timeline_entries", json!({}))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("timeline_id"), "{error}");

    let runtime = Runtime::try_new(
        RuntimeConfig::new(RuntimeDatabase::sqlite(dir.join("synd.db")))
            .with_runtime_root(dir.join("run")),
    )?;
    let session = runtime.acquire_session().await?;
    let timeline_id = session
        .client()
        .create_timeline(CreateTimelineInput {
            name: "reading".into(),
        })
        .await?
        .timeline_id;
    let timelines = peer.tool("list_timelines", json!({})).await?;
    assert_eq!(
        timelines,
        json!({"items": [{"id": timeline_id, "name": "reading"}]})
    );
    check_timeline_entry_pages(peer, session.client(), timeline_id).await?;
    session.close().await?;

    let error = peer
        .tool(
            "list_timeline_entries",
            json!({"timeline_id": timeline_id, "maxUnicodeWidth": 1001}),
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("maxUnicodeWidth"), "{error}");

    let page = peer
        .tool(
            "list_timeline_entries",
            json!({"timeline_id": TimelineId::nil()}),
        )
        .await?;
    assert_eq!(page, json!({"items": [], "next_cursor": null}));
    Ok(())
}

async fn check_timeline_entry_pages(
    peer: &mut McpPeer,
    client: &Client,
    timeline_id: TimelineId,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr = listener.local_addr()?;
    let twir_url = format!("http://{addr}/feed/twir_atom");
    let standalone_url = format!("http://{addr}/feed/standalone_atom_entry");
    synd_test::mock::spawn(listener);
    for url in [&twir_url, &standalone_url] {
        peer.tool("subscribe_feed", json!({"url": url})).await?;
    }

    // Four entries from This Week in Rust and one older, standalone Atom entry.
    let page = loop {
        let page = peer
            .tool(
                "list_timeline_entries",
                json!({"timeline_id": timeline_id, "first": 4}),
            )
            .await?;
        if page["next_cursor"].is_string() {
            break page;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    let entries = client
        .fetch_timeline_entries(
            TimelineEntriesRequest::builder()
                .timeline_id(timeline_id)
                .first(5)
                .build(),
        )
        .await?
        .nodes;
    assert_eq!(entries.len(), 5);

    let newest = &page["items"][0];
    assert_eq!(newest["id"], entries[0].entry.id.as_str());
    assert_eq!(newest["title"], "This Week in Rust 549");
    assert_eq!(
        newest["feed"],
        json!({"title": "This Week in Rust", "url": twir_url})
    );
    assert_eq!(
        newest["url"],
        "https://this-week-in-rust.org/blog/2024/05/29/this-week-in-rust-549/"
    );
    assert_eq!(newest["published"], "2024-05-29T04:00:00Z");
    let excerpt = newest["excerpt"].as_str().context("Missing excerpt")?;
    assert!(excerpt.starts_with("Hello and welcome to another issue of This Week in Rust"));
    assert_eq!(excerpt.len(), 200); // The start of this feed summary is ASCII.
    assert!(newest.get("summary").is_none());
    assert!(newest.get("content").is_none());
    let page = peer
        .tool(
            "list_timeline_entries",
            json!({"timeline_id": timeline_id, "first": 4, "after": page["next_cursor"], "maxUnicodeWidth": 5}),
        )
        .await?;
    assert_eq!(
        page,
        json!({
            "items": [{
                "id": entries[4].entry.id,
                "title": "Updated only",
                "excerpt": "Hello",
                "url": null,
                "feed": {"title": null, "url": standalone_url},
                "published": null,
                "orderTime": "2024-05-01T12:00:00Z"
            }],
            "next_cursor": null
        })
    );
    check_timeline_entry_time_range(peer, timeline_id).await
}

async fn check_timeline_entry_time_range(
    peer: &mut McpPeer,
    timeline_id: TimelineId,
) -> anyhow::Result<()> {
    fn order_times(page: &Value) -> Vec<&str> {
        page["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["orderTime"].as_str().unwrap())
            .collect()
    }

    let mut params = json!({
        "timeline_id": timeline_id,
        "since": "2024-05-15T13:00:00+09:00",
        "until": "2024-05-29T13:00:00+09:00",
        "first": 1,
    });
    let page = peer.tool("list_timeline_entries", params.clone()).await?;
    assert_eq!(order_times(&page), ["2024-05-22T04:00:00Z"]);
    params["after"] = json!(
        page["next_cursor"]
            .as_str()
            .context("Expected a second page in the time range")?
    );
    let page = peer.tool("list_timeline_entries", params).await?;
    assert_eq!(order_times(&page), ["2024-05-15T04:00:00Z"]);
    assert!(page["next_cursor"].is_null());

    for (mut bounds, expected) in [
        (
            json!({"since": "2024-05-29T04:00:00Z"}),
            vec!["2024-05-29T04:00:00Z"],
        ),
        (
            json!({"until": "2024-05-08T04:00:00Z"}),
            vec!["2024-05-01T12:00:00Z"],
        ),
        (
            json!({"since": "2024-05-22T04:00:00Z", "until": "2024-05-22T04:00:00Z"}),
            vec![],
        ),
    ] {
        bounds["timeline_id"] = json!(timeline_id);
        let page = peer.tool("list_timeline_entries", bounds).await?;
        assert_eq!(order_times(&page), expected);
        assert!(page["next_cursor"].is_null());
    }
    Ok(())
}

fn synd(dir: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_synd"));
    command
        .env("SYND_CONFIG_FILE", dir.join("synd.toml"))
        .env("SYND_SQLITE_DB", dir.join("synd.db"))
        .env("SYND_RUNTIME_ROOT", dir.join("run"))
        .env("SYND_LOG_FILE", dir.join("synd.log"))
        .env("SYND_ENABLE_GH", "false")
        .env("SYND_DAEMON_SESSION_IDLE_SHUTDOWN_GRACE", "30s")
        .kill_on_drop(true);
    command
}

async fn active_sessions(dir: &Path) -> anyhow::Result<u64> {
    let output = synd(dir)
        .args(["daemon", "status", "--output", "json"])
        .output()
        .await?;
    ensure!(
        output.status.success(),
        "daemon status failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let status: Value = serde_json::from_slice(&output.stdout)?;
    status["sessions"]["active_sessions"]
        .as_u64()
        .context("Invalid daemon status")
}

async fn shutdown_daemon(dir: &Path) -> anyhow::Result<()> {
    let output = synd(dir)
        .args(["daemon", "shutdown", "--force"])
        .output()
        .await?;
    ensure!(
        output.status.success(),
        "daemon shutdown failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

struct McpPeer {
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
}

impl McpPeer {
    async fn send(&mut self, message: Value) -> anyhow::Result<()> {
        self.stdin
            .write_all(format!("{message}\n").as_bytes())
            .await?;
        Ok(())
    }

    async fn request(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        self.send(json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}))
            .await?;
        let response = self
            .stdout
            .next_line()
            .await?
            .context("MCP stdout closed")?;
        let mut response: Value = serde_json::from_str(&response)?;
        assert_eq!(response["id"], 1);
        let result = response["result"].take();
        ensure!(result.is_object(), "MCP request failed: {response}");
        Ok(result)
    }

    async fn tool(&mut self, name: &str, arguments: Value) -> anyhow::Result<Value> {
        let mut result = self
            .request("tools/call", json!({"name": name, "arguments": arguments}))
            .await?;
        ensure!(result["isError"] != true, "MCP tool failed: {result}");
        Ok(result["structuredContent"].take())
    }
}
