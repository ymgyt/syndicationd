# CI

## GitHub Actions

### Secrets

| Secret               | Usage                           | GeneratedAt                                                      | ManagedBy | 
| ---                  | ---                             | ---                                                              | ---       |
| `CACHIX_AUTH_TOKEN`  | Read and Write cachix cache     | [cachix](https://app.cachix.org/personal-auth-tokens)            | @ymgyt    |
| `CODECOV_TOKEN`      | Upload test coverage to codecov | [codecov](https://app.codecov.io/gh/ymgyt/syndicationd/settings) | @ymgyt    |
| `HOMEBREW_TAP_TOKEN` | Push to [homebrew repo](https://github.com/ymgyt/homebrew-syndicationd/tree/main) by cargo-dist | [github](https://github.com/settings/tokens)       | @ymgyt | 
| `NPM_TOKEN`          | Push to [npm registry](https://www.npmjs.com/settings/syndicationd/packages) by cargo-dist      | [npm](https://www.npmjs.com/settings/ymgyt/tokens) | @ymgyt |


#### `HOMEBREW_TAP_TOKEN`

* [cargo-dist doc](https://opensource.axo.dev/cargo-dist/book/installers/homebrew.html)
* `repo` scope is required

#### `NPM_TOKEN`

* [cargo-dist doc](https://opensource.axo.dev/cargo-dist/book/installers/npm.html)
* Packages and scopes: Read and write
  * Select packages: All packages (NOTE: because the package does not yet exist, you must pick this. However, you can (and probably should!) update this to scope the token to a single package after publish. This is sadly a limitation of the npm token system.)
  * Organizations: No access

 
