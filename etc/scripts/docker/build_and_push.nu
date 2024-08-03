def main [tag: string] {
  let image = if ($tag | str starts-with 'synd-api-v' ) {
    let version = ($tag | str replace 'synd-api-v' '')
    {derivation: 'synd-api-image',version: $version }
  } else if ($tag | str starts-with 'synd-term-v' ) {
    let version = ($tag | str replace 'synd-term-v' '')
    {derivation: 'synd-term-image',version: $version }
  } else {
    print $"Ignore tag: ($tag)"
    return
  }

  # Build docker image
  print $"Build ($image.derivation)"
  nix build $".#($image.derivation)" --accept-flake-config

  # Load docker image
  print "Load image"
  docker load --input result

  # Tagging
  let package = ($image.derivation | str replace '-image' '')
  let src_tag = $"($package):latest"
  let dst_tag = $"ghcr.io/ymgyt/($package):($image.version)"
  let dst_tag_latest = $"ghcr.io/ymgyt/($package):latest"
  
  [$dst_tag $dst_tag_latest] | each {|tag| 
    docker tag $src_tag $tag
    print $"Push ($tag)"
    docker image push $tag
  }
}
