# Changelog

All notable changes to this project will be documented in this file.

## [unreleased] __release_date__

### Features

- Handle subscribe feed error by [@ymgyt](https://github.com/ymgyt) ([90c47d3f](https://github.com/ymgyt/syndicationd/commit/90c47d3f8e225cb71f33b1e6d6df0f0735e21f73))
- Use updated if published is none by [@ymgyt](https://github.com/ymgyt) ([9967dc10](https://github.com/ymgyt/syndicationd/commit/9967dc108f7f6602e321808398737f891462ec81))
- Add generator resolver in feed by [@ymgyt](https://github.com/ymgyt) ([f8de4aa4](https://github.com/ymgyt/syndicationd/commit/f8de4aa4a9a4edb8d1f7e8dd31c53b2e66360b18))
- Resolve entry content if there is no summary by [@ymgyt](https://github.com/ymgyt) ([0459e71c](https://github.com/ymgyt/syndicationd/commit/0459e71c38aba96b4d878ce97cd35ed78587032b))

## [synd-api-v0.1.2] - 2024-02-20

### Features

- Raise soft fd limit by [@ymgyt](https://github.com/ymgyt) ([54e7ba3c](https://github.com/ymgyt/syndicationd/commit/54e7ba3c44a4a379e61edea95bc27c487fa0b7d6))
- Instrument kvsd client span name by [@ymgyt](https://github.com/ymgyt) ([bced1b62](https://github.com/ymgyt/syndicationd/commit/bced1b62a52e79e0af70fb2177a2efa940adf91c))
- Use monotonic_counter as feed subscription metrics by [@ymgyt](https://github.com/ymgyt) ([670dc430](https://github.com/ymgyt/syndicationd/commit/670dc4300310695a71ee73db90f066309323ba6b))

### Bug Fixes

- Strict fetch feed in flight limit by [@ymgyt](https://github.com/ymgyt) ([5a2b646e](https://github.com/ymgyt/syndicationd/commit/5a2b646e2d4fa2b24a2f115a27288c922fa87af3))

## [synd-api-v0.1.1] - 2024-02-19

### Features

- Add o11y crate by [@ymgyt](https://github.com/ymgyt) ([0a50517e](https://github.com/ymgyt/syndicationd/commit/0a50517e0b861973fac95ad5dba6f2c4d5b7270d))
- Add opentelemetry-tracing log bridge layer by [@ymgyt](https://github.com/ymgyt) ([92f22b56](https://github.com/ymgyt/syndicationd/commit/92f22b564357a0d43f8631212cf976338eb05a04))
- Add baggage propagation by [@ymgyt](https://github.com/ymgyt) ([d02e514c](https://github.com/ymgyt/syndicationd/commit/d02e514c8f6e32aa748c10dadb204153cba21ecc))
- Add opentelemetry layers by [@ymgyt](https://github.com/ymgyt) ([4d3f5bf3](https://github.com/ymgyt/syndicationd/commit/4d3f5bf3f45f31cfd014dbdf37a41a31ea0472ca))
- Add palette flag by [@ymgyt](https://github.com/ymgyt) ([04dc486d](https://github.com/ymgyt/syndicationd/commit/04dc486d0ab3043e021e164e70f5fe081e3c464d))
- Impl kvsd client by [@ymgyt](https://github.com/ymgyt) ([6ae6de7a](https://github.com/ymgyt/syndicationd/commit/6ae6de7a2e783417b1a8d5d3c2b450109d83725f))
- Use kvsd by [@ymgyt](https://github.com/ymgyt) ([19eaeada](https://github.com/ymgyt/syndicationd/commit/19eaeadab75be9ea0c7c95e65ca654f9842707af))
- Remove unsubscribed entries by [@ymgyt](https://github.com/ymgyt) ([d29ba92e](https://github.com/ymgyt/syndicationd/commit/d29ba92e929d9d1348fa114ac2bdf210b76c5a1b))
- Serve https by [@ymgyt](https://github.com/ymgyt) ([fbb9011e](https://github.com/ymgyt/syndicationd/commit/fbb9011e86acf6e4cf30f37a74e67d3202bbc5a0))
- Support axum_server graceful shutdown by [@ymgyt](https://github.com/ymgyt) ([880b6d3e](https://github.com/ymgyt/syndicationd/commit/880b6d3e8d0f90b711a1d6e8e1bf6fb1808e5161))
- Instrument request counter metrics by [@ymgyt](https://github.com/ymgyt) ([ac64b3aa](https://github.com/ymgyt/syndicationd/commit/ac64b3aa6880482597e672649de800eb30b3ad56))
- Export basic counter metrics by [@ymgyt](https://github.com/ymgyt) ([13ba79b7](https://github.com/ymgyt/syndicationd/commit/13ba79b7a20f5b9b573e7285a02302d8dc848b03))
- Add fallback handler by [@ymgyt](https://github.com/ymgyt) ([681d0315](https://github.com/ymgyt/syndicationd/commit/681d0315b49c1b2a157d3141f0e45be95e32272e))
- Remove path attribute from http request count metrics by [@ymgyt](https://github.com/ymgyt) ([017470e5](https://github.com/ymgyt/syndicationd/commit/017470e50f38b26270cfa9e3c1d85a588b23e725))

### Miscellaneous Tasks

- Format toml by [@ymgyt](https://github.com/ymgyt) ([36677745](https://github.com/ymgyt/syndicationd/commit/3667774506106fe0f38d77efac9f4b27c70090aa))
- Organize dev files by [@ymgyt](https://github.com/ymgyt) ([4af5df57](https://github.com/ymgyt/syndicationd/commit/4af5df57a38f69b734b3e4ceaf741b3415bed6e1))
- Configure release flow by [@ymgyt](https://github.com/ymgyt) ([855d1063](https://github.com/ymgyt/syndicationd/commit/855d1063f5b476433fe0a7ab352b72d63a749e2e))
- Use hyphen as package name instead of underscore by [@ymgyt](https://github.com/ymgyt) ([0a8ed059](https://github.com/ymgyt/syndicationd/commit/0a8ed05997790f9f05c932c92fa2b2b2d74065a9))
- Instrument by [@ymgyt](https://github.com/ymgyt) ([07839dc1](https://github.com/ymgyt/syndicationd/commit/07839dc10e7c44cae79055eea6103f099f0daf5e))
- Add bin section to Cargo.toml by [@ymgyt](https://github.com/ymgyt) ([9bfd56ef](https://github.com/ymgyt/syndicationd/commit/9bfd56ef41e27f094ef240653da47cdda662d2fb))

### Refactor

- Rename crates by [@ymgyt](https://github.com/ymgyt) ([ce0982e4](https://github.com/ymgyt/syndicationd/commit/ce0982e497647b23dcf07e39d525121bcd9ac1fa))
- Use clippy pedantic by [@ymgyt](https://github.com/ymgyt) ([328ddade](https://github.com/ymgyt/syndicationd/commit/328ddadebbad5381271c5e84cce2d6888252e70c))
- Rename datastore to repository by [@ymgyt](https://github.com/ymgyt) ([969c0052](https://github.com/ymgyt/syndicationd/commit/969c0052164a7719d5c8902a5fd70b40c42faae5))
- Rename to subscription repository by [@ymgyt](https://github.com/ymgyt) ([707ec5f3](https://github.com/ymgyt/syndicationd/commit/707ec5f3197b6079e420a8c5e2dc17c3efd7ed56))
- Rename to repository by [@ymgyt](https://github.com/ymgyt) ([aed9bb48](https://github.com/ymgyt/syndicationd/commit/aed9bb4873c2a286699898e2c37825e292811ee6))
- Fix lint by [@ymgyt](https://github.com/ymgyt) ([aac00b98](https://github.com/ymgyt/syndicationd/commit/aac00b98335bb75cc57fdea0875bfd675bf8f3cc))

### Testing

- Impl device flow test case by [@ymgyt](https://github.com/ymgyt) ([93572902](https://github.com/ymgyt/syndicationd/commit/9357290265a4fbf8d78721e4f9f1904b1cf5b12a))
- Add auth flow case by [@ymgyt](https://github.com/ymgyt) ([6d2b1905](https://github.com/ymgyt/syndicationd/commit/6d2b1905d9b06bd9ed670f210cd590f89405c37c))
- Run integration test by [@ymgyt](https://github.com/ymgyt) ([20c0bc2d](https://github.com/ymgyt/syndicationd/commit/20c0bc2d31a938d3103fafedba5a10b4a9bba9ae))

<!-- generated by git-cliff -->
