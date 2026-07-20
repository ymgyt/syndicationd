set shell := ["nu", "-c"]

mod dep 'etc/just/dep.just'
mod doc 'etc/just/doc.just'
mod dot 'etc/just/dot.just'
mod etc 'etc/just/etc'
mod nix 'etc/just/nix'
mod run 'etc/just/run.just'
mod demo 'etc/just/demo.just'
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

# Generate product changelog
changelog target:
    nu "{{justfile_directory() / 'etc/scripts/changelog.nu'}}" "{{target}}"

# List recipe
default:
    just --list --list-submodules
