use synd_test::temp_dir;

#[test]
#[ignore = "requires runtime daemon endpoint creation"]
fn local_port_commands() {
    let dir = temp_dir().keep();
    let sqlite_db = dir.join("synd.db");
    let input = dir.join("subscriptions.json");
    std::fs::write(&input, r#"{"feeds":[]}"#).unwrap();

    let sqlite_db = sqlite_db.display().to_string();
    let input = input.display().to_string();

    let check = assert_cmd::Command::cargo_bin("synd")
        .unwrap()
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

    assert_cmd::Command::cargo_bin("synd")
        .unwrap()
        .args(["--sqlite-db", &sqlite_db, "feed", "export"])
        .assert()
        .success();

    assert_cmd::Command::cargo_bin("synd")
        .unwrap()
        .args(["--sqlite-db", &sqlite_db, "feed", "import", &input])
        .assert()
        .success();
}

#[test]
fn check_command() {
    let mut cmd = assert_cmd::Command::cargo_bin("synd").unwrap();
    let dir = temp_dir().keep();
    let sqlite_db = dir.join("synd.db").display().to_string();

    cmd.args(["--sqlite-db", &sqlite_db, "config", "view"])
        .assert()
        .success();

    cmd.arg("--output=json").assert().success();
}

#[test]
#[ignore = "requires runtime daemon endpoint creation"]
fn export_command() {
    let mut cmd = assert_cmd::Command::cargo_bin("synd").unwrap();
    let dir = temp_dir().keep();
    let sqlite_db = dir.join("synd.db").display().to_string();

    cmd.args(["--sqlite-db", &sqlite_db, "feed", "export"])
        .assert()
        .success();

    cmd.arg("--print-schema").assert().success();
}

#[test]
fn clean_command() {
    let cache_dir = temp_dir().keep();
    let mut cmd = assert_cmd::Command::cargo_bin("synd").unwrap();

    cmd.args(["--cache-dir", &cache_dir.display().to_string(), "clean"])
        .assert()
        .success();
}
