use std::path::Path;

use synd_test::temp_dir;

fn command(root: &Path) -> assert_cmd::Command {
    let mut command = assert_cmd::Command::cargo_bin("synd").unwrap();
    command
        .env_clear()
        .current_dir(root)
        .env("HOME", root)
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_CACHE_HOME", root.join("cache"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_RUNTIME_DIR", root)
        .env("SYND_LOG_FILE", root.join("synd.log"));
    command
}

#[test]
fn redirected_terminal_leaves_the_database_runtime_and_filter_cache_untouched() {
    let dir = temp_dir();
    let root = dir.path();
    let config = root.join("config.toml");
    let cache = root.join("cache");
    let database = root.join("feeds.db");
    let runtime = root.join("daemon");
    std::fs::write(&config, "").unwrap();
    std::fs::create_dir(&cache).unwrap();
    let settings = cache.join(synd_term::config::cache::GH_NOTIFICATION_FILTER_OPTION_FILE);
    std::fs::write(&settings, "preserve the original contents").unwrap();

    let output = command(root)
        .arg("--config")
        .arg(config)
        .arg("--cache-dir")
        .arg(cache)
        .arg("--sqlite-db")
        .arg(&database)
        .arg("--runtime-root")
        .arg(&runtime)
        .args(["--enable-gh", "true", "--github-pat", "test-token"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.contains("stdout is not a terminal"), "{stderr}");
    assert!(
        stderr.contains(&format!("see log: {}", root.join("synd.log").display())),
        "{stderr}"
    );
    let log = std::fs::read_to_string(root.join("synd.log")).unwrap();
    assert!(log.contains("stdout is not a terminal"), "{log}");
    assert!(!database.exists());
    assert!(!runtime.exists());
    assert_eq!(
        std::fs::read_to_string(settings).unwrap(),
        "preserve the original contents"
    );
}

#[cfg(unix)]
#[test]
#[ignore = "requires runtime daemon endpoint creation"]
fn feed_commands_release_their_daemon_sessions() {
    let temp = temp_dir();
    let dir = temp.path();
    let sqlite_db = dir.join("synd.db");
    let input = dir.join("subscriptions.json");
    std::fs::write(&input, r#"{"feeds":[]}"#).unwrap();

    let sqlite_db = sqlite_db.display().to_string();
    let input = input.display().to_string();

    let check = command(dir)
        .args(["--sqlite-db", &sqlite_db, "config", "view"])
        .assert()
        .success();
    let output = check.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("  SQLite DB: {sqlite_db}")));
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    command(dir)
        .args(["--sqlite-db", &sqlite_db, "feed", "export"])
        .assert()
        .success();

    command(dir)
        .args(["--sqlite-db", &sqlite_db, "feed", "import", &input])
        .assert()
        .success();

    let url = "http://127.0.0.1:9/feed";
    command(dir)
        .args(["--sqlite-db", &sqlite_db, "feed", "subscribe", "--url", url])
        .assert()
        .success()
        .stdout(format!("{url} subscription recorded.\n"));
    let mut unsubscribe = command(dir);
    unsubscribe.args([
        "--sqlite-db",
        &sqlite_db,
        "feed",
        "unsubscribe",
        "--url",
        url,
    ]);
    unsubscribe
        .assert()
        .success()
        .stdout(format!("{url} subscription removed.\n"));
    unsubscribe.assert().code(1);

    let output = command(dir)
        .args([
            "--sqlite-db",
            &sqlite_db,
            "daemon",
            "status",
            "--output=json",
        ])
        .assert()
        .success();
    let status: serde_json::Value = serde_json::from_slice(&output.get_output().stdout).unwrap();
    assert_eq!(status["sessions"]["active_sessions"], 0);
    command(dir)
        .args(["--sqlite-db", &sqlite_db, "daemon", "shutdown"])
        .assert()
        .success();
}

#[test]
fn check_command() {
    let dir = temp_dir();
    let mut cmd = command(dir.path());
    let sqlite_db = dir.path().join("synd.db").display().to_string();

    cmd.args(["--sqlite-db", &sqlite_db, "config", "view"])
        .assert()
        .success();

    cmd.arg("--output=json").assert().success();
}

#[cfg(unix)]
#[test]
fn doctor_checks_the_current_directory_for_bare_file_names() {
    let dir = temp_dir();
    let output = command(dir.path())
        .args([
            "--sqlite-db",
            "feeds.db",
            "--log",
            "synd.log",
            "doctor",
            "--output=json",
        ])
        .assert()
        .success();
    let report: serde_json::Value = serde_json::from_slice(&output.get_output().stdout).unwrap();
    let checks = report["checks"].as_array().unwrap();
    for name in ["sqlite_parent", "log_parent"] {
        let check = checks.iter().find(|check| check["name"] == name).unwrap();
        assert_eq!(check["status"], "pass");
        assert_eq!(check["path"], ".");
    }
}

#[test]
fn doctor_returns_failure_in_its_report_when_a_check_fails() {
    let dir = temp_dir();
    let output = command(dir.path())
        .env("SYND_LOG", "off")
        .arg("--sqlite-db")
        .arg(dir.path())
        .args(["doctor", "--output=json"])
        .assert()
        .code(1)
        .stderr("");
    let report: serde_json::Value = serde_json::from_slice(&output.get_output().stdout).unwrap();
    assert!(
        report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["name"] == "sqlite_db" && check["status"] == "fail")
    );
}

#[cfg(unix)]
#[test]
fn daemon_status_resolves_the_default_database_before_first_use() {
    let dir = temp_dir();
    let output = command(dir.path())
        .args(["daemon", "status", "--output=json"])
        .assert()
        .success();
    let status: serde_json::Value = serde_json::from_slice(&output.get_output().stdout).unwrap();
    assert_eq!(status["state"], "not-running");
    let database = Path::new(status["placement"]["database"].as_str().unwrap());
    assert!(database.starts_with(dir.path().canonicalize().unwrap()));
    assert!(database.ends_with("synd.db"));
    assert!(!database.parent().unwrap().exists());
}

#[test]
#[ignore = "requires runtime daemon endpoint creation"]
fn export_command() {
    let dir = temp_dir();
    let mut cmd = command(dir.path());
    let sqlite_db = dir.path().join("synd.db").display().to_string();

    cmd.args(["--sqlite-db", &sqlite_db, "feed", "export"])
        .assert()
        .success();

    cmd.arg("--print-schema").assert().success();
}

#[test]
fn clean_command() {
    let dir = temp_dir();
    let log = dir.path().join("synd.log");
    std::fs::write(&log, "test log").unwrap();

    command(dir.path()).arg("clean").assert().success();

    assert!(!log.exists());
}

#[test]
fn clean_reports_the_cause_once_when_tracing_is_disabled() {
    let dir = temp_dir();
    let log = dir.path().join("log-directory");
    std::fs::create_dir(&log).unwrap();
    let output = command(dir.path())
        .env("SYND_LOG", "off")
        .arg("--log")
        .arg(&log)
        .args(["clean", "--logs"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(
        stderr.starts_with(&format!(
            "error: failed to remove log file {}",
            log.display()
        )),
        "{stderr}"
    );
    assert!(log.is_dir());
}
