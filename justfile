set shell := ["nu", "-c"]

mod graphql 'etc/just/graphql.just'
mod run 'etc/just/run.just'
mod doc 'etc/just/doc.just'
mod format 'etc/just/format.just'
mod check 'etc/just/check.just'
mod lint 'etc/just/lint.just'
mod test 'etc/just/test.just'
mod audit 'etc/just/audit.just'
mod bench 'etc/just/bench.just'
mod dep 'etc/just/dep.just'
mod license 'etc/just/license.just'
mod dist 'etc/just/dist.just'
mod oranda 'etc/just/oranda.just'
mod demo 'etc/just/demo.just'
mod bpf 'etc/just/bpf.just'
mod etc 'etc/just/etc'
mod dot 'etc/just/dot.just'
mod synd 'etc/just/synd.just'
mod tf 'hosting/terraform'

# List recipe
default:
    just --list --list-submodules
