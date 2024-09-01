## Development

In syndicationd, we manage the development environment and CI with [Nix](https://nixos.org/).  
For installing Nix, please refer to the [Nix install documentation](https://github.com/DeterminateSystems/nix-installer)

By executing `nix develop`, the necessary tools for development can be prepared.

### Overview of Packages

![Overview](etc/dot/packages.svg)

| Package     | Description        |
| ---         | ---                |
| `synd_o11y` | Observability lib  |
| `synd_feed` | RSS/Atom feed lib  |
| `synd_auth` | Authentication lib |
| `synd_api`  | GraphQL api bin    |
| `synd_term` | TUI app bin        |

### Launching Develop Environment

Launch three terminals.  

1. the first one is for synd-api
1. the second one is for kvsd, which serves as the persistent backend used by synd-api
1. the last one is for synd.

Execute in the following order: `just run kvsd`, `just run api`, `just run term`

### Updating GraphQL Schema

If you have updated the GraphQL schema of synd-api, execute `just graphql schema` This command updates the GraphQL schema of synd-term.   
To generate synd-term's GraphQL client code, execute `just graphql generate`.

## Testing

* `just lint` : run linters
* `just test unit` : run unit tests
* `just test integration` : run integration test
* `just bench` : run benchmarks
* `just bench flamegraph` : generate flamegraph


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

Please use the scope without `synd` prefix from the crate name.  
For example, when making changes to `synd_term`, the commit message should be `feat(term): add new feature`.  
The commit will be used to generate the CHANGELOG for each crate.

## For Maintainers

For information about CI, please refer to [ci.md](/docs/ci.md).  

### Release Flow

To perform a release, run `just <package> release (patch|minor|major) [--execute]`.  
For example, to release version v0.2.0 of `synd-api` when it is currently at version v0.1.0, you would run `just synd api release minor`.  

This task will be executed in dry-run mode, allowing you to review the CHANGELOG generation and replacement processing. Once you have confirmed that there are no issues, return the command with the `--execute` flag.  

This process will publish the package to crates.io and push the git tag.  
The git tag will trigger the release workflow, which will create a GitHub release.

## License

By contributing to `syndicationd`, you agree that your contributions will be dual-licensed under
the terms of the [`LICENSE-MIT`](./LICENSE-MIT) and [`LICENSE-APACHE`](./LICENSE-APACHE) files in the
root directory of this source tree.
