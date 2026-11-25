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
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("stdout is not a terminal"),
        "{:?}",
        output.stderr
    );
    assert!(!database.exists());
    assert!(!runtime.exists());
    assert_eq!(
        std::fs::read_to_string(settings).unwrap(),
        "preserve the original contents"
    );
}

#[test]
#[ignore = "requires runtime daemon endpoint creation"]
fn local_port_commands() {
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
