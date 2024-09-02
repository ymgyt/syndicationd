def main [
  filter?: string
  ] {

  # When executing part of the test and specifying "delete" for unreferenced
  # the snapshots of the filtered test cases are deleted. 
  # Therefore, if a filter is specified, set it to "ignore".
  let unreferenced = if ($filter == null) {
    "warn"
  } else {
    "ignore"
  }

  $env.RUST_LOG = "synd_term=info,integration=info,synd_test=info,synd_feed=warn,kvsd=warn,metrics=warn,tower_http=warn,info"
  $env.INSTA_OUTPUT = "diff"
  $env.INSTA_UPDATE = "new" 
  cd crates/synd_term
  (
    cargo insta test 
      --package synd-term
      --features "integration"
      --test integration
      --unreferenced $unreferenced
      --test-runner "nextest"
      --review
      --
      ($filter | into string)
  )
}
