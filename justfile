set shell := ["nu", "-c"]

mod dep 'etc/just/dep.just'
mod doc 'etc/just/doc.just'
mod dot 'etc/just/dot.just'
mod etc 'etc/just/etc'
mod nix 'etc/just/nix'
mod run 'etc/just/run.just'
mod dist 'etc/just/dist.just'
mod lint 'etc/just/lint.just'
mod synd 'etc/just/synd.just'
mod test 'etc/just/test.just'
mod audit 'etc/just/audit.just'
mod bench 'etc/just/bench.just'
mod check 'etc/just/check.just'
mod format 'etc/just/format.just'
mod oranda 'etc/just/oranda.just'
mod graphql 'etc/just/graphql.just'
mod license 'etc/just/license.just'

# List recipe
default:
    just --list --list-submodules

# Record the README GIF and GitHub social preview on Wayland
demo:
    let wlrec = ($env.WLREC_DIR? | default '../wlrec'); nix develop $wlrec --command nix shell 'nixpkgs#jq' 'nixpkgs#gifsicle' 'nixpkgs#ffmpeg-headless' --command bash etc/demo/record.sh $wlrec

# Generate product changelog
changelog target:
    nu "{{justfile_directory() / 'etc/scripts/changelog.nu'}}" "{{target}}"

