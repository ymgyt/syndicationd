# Contributing

## Development

syndicationd uses [Nix](https://nixos.org/) to prepare the development and CI
toolchain. For installing Nix, refer to the
[Nix install documentation](https://github.com/DeterminateSystems/nix-installer).

Enter the development shell before running the project commands:

```sh
nix develop
```

### Overview of Packages

![Overview](etc/dot/dist/packages.svg)

| Package             | Description                                  |
| ---                 | ---                                          |
| `synd`             | Binary crate and composition root           |
| `synd_support`     | Shared support and observability utilities  |
| `synd_feed`        | RSS/Atom feed lib                           |
| `synd_auth`        | Authentication lib                          |
| `synd_registry`    | Feed registry domain lib                    |
| `synd_persistence` | SQLite storage adapters                     |
| `synd_api`         | GraphQL API server                          |
| `synd_client`      | API client and payload DTOs                 |
| `synd_runtime`     | Runtime session and local API lifecycle     |
| `synd_term`        | TUI application                             |
| `synd_test`        | Test support lib                            |

### Application Topology

![Application overview](etc/dot/dist/overview.svg)

For normal development and user workflows, `synd` is the composition root. It
resolves CLI/configuration, acquires a `synd_runtime::Session`, and passes the
session's `synd_client::Client` into `synd_term`.

`synd_runtime` owns local API lifecycle concerns. The current implementation
starts a loopback local `synd_api` backed by SQLite as the transitional runtime
path. The singleton daemon, startup lock, and UDS transport should be added
behind the same `Runtime`/`Session` API.

`synd_term` owns the TUI application and event loop. It should receive an
already configured `Client`; it must not start local API services or own runtime
session lifecycle.

The standalone `synd_api` binary is still useful when working on the API server
itself or updating the GraphQL schema.

### Running Locally

The normal development path is the in-process local backend:

```sh
just run term
```

This starts the `synd` binary, which acquires a runtime session and then starts
the TUI.

List available recipes with:

```sh
just
```

### Updating GraphQL Schema

If you update the `synd-api` GraphQL schema, update the schema used by
`synd-client` and regenerate the client code:

```sh
just graphql schema
just graphql generate
```

`just graphql schema` reads `GH_PAT` when the introspection request needs an
authorization header.

## Testing

* `just lint`: run typo and clippy checks
* `just test`: run unit and integration tests
* `just test unit`: run unit tests with nextest
* `just test integration`: run integration tests
* `just bench`: run benchmarks
* `just bench flamegraph`: generate a flamegraph

For `synd-term` changes that touch feature-gated code, run both:

```sh
cargo clippy -p synd-term -- -D warnings
cargo clippy -p synd-term --all-features -- -D warnings
```

## Commit Message

Commit message should follow [conventional commit](https://www.conventionalcommits.org/en/v1.0.0/).  
type is one of the following.

| commit type | description                         |
|-------------|-------------------------------------|
| `feat`      | add a new feature                   |
| `style`     | tui style                           |
| `fix`       | bug fix                             |
| `perf`      | performance improvement             |
| `doc`       | documentation                       |
| `ci`        | continuous Integration and delivery |
| `refactor`  | refactoring                         |
| `chore`     | catch all                           |

Use the scope without the `synd` prefix from the crate name.
For example, when making changes to `synd_term`, the commit message should be `feat(term): add new feature`.
The commit will be used to generate the CHANGELOG for each crate.

## For Maintainers

For information about CI, refer to [ci.md](/docs/ci.md).

### Release Flow

To perform a release, run `just synd <package> release (patch|minor|major) [--execute]`.
For example, to release version v0.2.0 of `synd-api` when it is currently at version v0.1.0, run `just synd api release minor`.

This task will be executed in dry-run mode, allowing you to review the CHANGELOG generation and replacement processing. Once you have confirmed that there are no issues, return the command with the `--execute` flag.  

This process will publish the package to crates.io and push the git tag.  
The git tag will trigger the release workflow, which will create a GitHub release.

### Update rust version

The project's rust version is managed in the [rust-toolchain.toml](./rust-toolchain.toml).  
If you encounter the following error after upgrading the Rust version and running `nix develop`:

```
error: Stable 1.x.y is not available  
```

In that case, execute `just nix update rust-overlay`.

## License

By contributing to `syndicationd`, you agree that your contributions will be dual-licensed under
the terms of the [`LICENSE-MIT`](./LICENSE-MIT) and [`LICENSE-APACHE`](./LICENSE-APACHE) files in the
root directory of this source tree.
