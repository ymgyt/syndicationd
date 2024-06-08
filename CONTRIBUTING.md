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


## Development

In syndicationd, we manage the development environment with [Nix](https://nixos.org/).  
By executing `nix develop`, the necessary tools for development can be prepared.

To test code, execute `just check`

### Launching Develop Environment

Launch three terminals.  

1. the first one is for synd-api
1. the second one is for kvsd, which serves as the persistent backend used by synd-api
1. the last one is for synd.

Execute in the following order: `just kvsd`, `just api`, `just term`

### Updating GraphQL Schema

If you have updated the GraphQL schema of synd-api, execute `just update-gql-schema` This command updates the GraphQL schema of synd-term.   
To generate synd-term's GraphQL client code, execute `just gen-gql`.

## License

By contributing to `syndicationd`, you agree that your contributions will be dual-licensed under
the terms of the [`LICENSE-MIT`](./LICENSE-MIT) and [`LICENSE-APACHE`](./LICENSE-APACHE) files in the
root directory of this source tree.
