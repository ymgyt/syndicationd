#![cfg(unix)]

use std::{path::Path, process::Stdio, time::Duration};

use anyhow::{Context as _, ensure};
use serde_json::{Value, json};
use synd_client::payload::{CreateTimelineInput, TimelineId};
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
    let timeline_entries = definitions
        .iter()
        .find(|tool| tool["name"] == "list_timeline_entries")
        .unwrap();
    assert!(timeline_entries["inputSchema"]["properties"]["maxUnicodeWidth"].is_object());
    let mut tools: Vec<&str> = definitions
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    tools.sort_unstable();
    assert_eq!(
        tools,
        [
            "list_subscriptions",
            "list_timeline_entries",
            "list_timelines",
            "subscribe_feed",
            "unsubscribe_feed"
        ]
    );

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
    let created = session
        .client()
        .create_timeline(CreateTimelineInput {
            name: "reading".into(),
        })
        .await?;
    session.close().await?;

    let timelines = peer.tool("list_timelines", json!({})).await?;
    assert_eq!(
        timelines,
        json!({"items": [{"id": created.timeline_id, "name": "reading"}]})
    );
    let id = &timelines["items"][0]["id"];

    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!("http://{}/feed/twir_atom", listener.local_addr()?);
    synd_test::mock::spawn(listener);
    peer.tool("subscribe_feed", json!({"url": url})).await?;

    // Wait for the daemon to project all four entries from the Atom feed.
    let page = loop {
        let page = peer
            .tool(
                "list_timeline_entries",
                json!({"timeline_id": id, "first": 3}),
            )
            .await?;
        if page["next_cursor"].is_string() {
            break page;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    assert_eq!(page["items"][0]["title"], "This Week in Rust 549");
    let excerpt = page["items"][0]["excerpt"]
        .as_str()
        .context("Missing excerpt")?;
    assert!(excerpt.starts_with("Hello and welcome to another issue of This Week in Rust"));
    assert_eq!(excerpt.len(), 200); // The start of this feed summary is ASCII.
    assert!(page["items"][0].get("summary").is_none());
    let page = peer
        .tool(
            "list_timeline_entries",
            json!({"timeline_id": id, "first": 3, "after": page["next_cursor"], "maxUnicodeWidth": 5}),
        )
        .await?;
    assert_eq!(page["items"][0]["title"], "This Week in Rust 546");
    assert_eq!(page["items"][0]["excerpt"], "Hello");
    assert!(page["next_cursor"].is_null());

    let error = peer
        .tool(
            "list_timeline_entries",
            json!({"timeline_id": id, "maxUnicodeWidth": 1001}),
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
