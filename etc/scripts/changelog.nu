use std log info

# Generate CHANGELOG
def main [
  package: string # Package identirifer. ex api for synd_api package
  version: string # Latest version in CHANGELOG
] {
  # make sure version have prefix 'v'
  let version = ( $version | str trim --left --char 'v' | ['v', $in ] | str join)
  let root = "../.."
  # handle package rename
  let additional_include_path = if ($package == "auth") {
    "crates/synd_authn/**"
  } else {
    ""
  }

  info $"Generate CHANGELOG.md for synd-($package) ($version)"

  (git cliff
    --config $"($root)/cliff.toml"
    --repository $root
    --include-path $"crates/synd_($package)/**" $additional_include_path
    --tag-pattern $"synd-($package)-v.*"
    --tag $"synd-($package)-($version)"
    --output $"($root)/crates/synd_($package)/CHANGELOG.md"
  )
}
