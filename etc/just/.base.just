set shell := ["nu", "-c"]

os := if os() == "macos" { "darwin" } else { "linux" }
term_dir := "crates/synd_term"
auth_dir := "crates/synd_auth"
feed_dir := "crates/synd_feed"
o11y_dir := "crates/synd_o11y"
api_dir := "crates/synd_api"
