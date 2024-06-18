# Changelog

All notable changes to this project will be documented in this file.

## [v0.1.7] 2024-06-18

### Miscellaneous Tasks

- Update opentelemetry from 0.22.0 to 0.23.0 by [@ymgyt](https://github.com/ymgyt) ([1f291fc3](https://github.com/ymgyt/syndicationd/commit/1f291fc31ec07d5f84565518d848b0822cccb879))

### Refactor

- Remove empty line by [@ymgyt](https://github.com/ymgyt) ([ebafc736](https://github.com/ymgyt/syndicationd/commit/ebafc7364333e212bbd56e5e536c7717a96c749b))

### Testing

- Add tracing opentelemetry layer tests (#50) by [@Yuta Yamaguchi](https://github.com/Yuta Yamaguchi) ([9739b518](https://github.com/ymgyt/syndicationd/commit/9739b5189bdeaedd458771a92a04e416f1af08fd))
- Add metrics view test case by [@ymgyt](https://github.com/ymgyt) ([9384adf3](https://github.com/ymgyt/syndicationd/commit/9384adf336db680e50a380ab2f42b4113108f691))
- Add http extractor test case by [@ymgyt](https://github.com/ymgyt) ([87cbd5f1](https://github.com/ymgyt/syndicationd/commit/87cbd5f1d817f0e48f570b895d6c533b99172e30))

## [v0.1.6] - 2024-05-06

### Documentation

- Add comments to public api by [@ymgyt](https://github.com/ymgyt) ([41588384](https://github.com/ymgyt/syndicationd/commit/41588384a0344a3befc473b9ca45abe6be2054ac))

## [v0.1.5] - 2024-04-17

### Features

- Add opentelemetry-stdout feature by [@ymgyt](https://github.com/ymgyt) ([765c036d](https://github.com/ymgyt/syndicationd/commit/765c036dc143f976e108935943bd9f89f03deea7))

### Miscellaneous Tasks

- Change view event level to debug by [@ymgyt](https://github.com/ymgyt) ([1e8e0c07](https://github.com/ymgyt/syndicationd/commit/1e8e0c07cffd48dc42e202185f42a0afdfd2fd03))

### Refactor

- Compose otel layer by [@ymgyt](https://github.com/ymgyt) ([3562f3f0](https://github.com/ymgyt/syndicationd/commit/3562f3f0eb6224f89181ab6af87ec0b1c2e2403c))
- Use semantic conventions const by [@ymgyt](https://github.com/ymgyt) ([2f1bac7d](https://github.com/ymgyt/syndicationd/commit/2f1bac7d407d32078ce48ef8a495fa2deecbb9d2))

## [v0.1.4] - 2024-03-08

### Miscellaneous Tasks

- Disable cargo-dist by [@ymgyt](https://github.com/ymgyt) ([d7cf038f](https://github.com/ymgyt/syndicationd/commit/d7cf038f329f43645d49667b923125879afb8e1c))
- Remove workspace otel deps by [@ymgyt](https://github.com/ymgyt) ([20c071ea](https://github.com/ymgyt/syndicationd/commit/20c071ea08d1c4afc1cd0a724037bbdfa10eb1cf))
- Update opentelemetry from 0.21 to 0.22 by [@ymgyt](https://github.com/ymgyt) ([b874c1d5](https://github.com/ymgyt/syndicationd/commit/b874c1d5ba804339a495a29a28cfd6443b2e2339))

## [v0.1.3] - 2024-02-25

### Features

- Add health check response type by [@ymgyt](https://github.com/ymgyt) ([f2420630](https://github.com/ymgyt/syndicationd/commit/f242063027b2ba5cac06a871a4c24d2413366cf4))
- Impl fmt::Display for health check status by [@ymgyt](https://github.com/ymgyt) ([b52bd8d5](https://github.com/ymgyt/syndicationd/commit/b52bd8d56d39bc4263f0c4851fb078803bd65881))

### Miscellaneous Tasks

- Trim prefix from changelog by [@ymgyt](https://github.com/ymgyt) ([95d44877](https://github.com/ymgyt/syndicationd/commit/95d448773ec7ab009fbece0928854364679b6f2c))
- Add homepage to package metadata by [@ymgyt](https://github.com/ymgyt) ([4bfdb49e](https://github.com/ymgyt/syndicationd/commit/4bfdb49e317e18ff6345ce1b8e8071f0497a1a5f))

## [v0.1.2] - 2024-02-19

### Features

- Instrument request counter metrics by [@ymgyt](https://github.com/ymgyt) ([ac64b3aa](https://github.com/ymgyt/syndicationd/commit/ac64b3aa6880482597e672649de800eb30b3ad56))
- Add metric macro by [@ymgyt](https://github.com/ymgyt) ([5b723f4b](https://github.com/ymgyt/syndicationd/commit/5b723f4b0c68b422f4778b502d2136ef4662bebd))
- Configure endpoint by [@ymgyt](https://github.com/ymgyt) ([14e09265](https://github.com/ymgyt/syndicationd/commit/14e0926596c59a5e32c283d0f8ac7f805e9e97d9))

### Documentation

- Fix typo by [@ymgyt](https://github.com/ymgyt) ([c311417b](https://github.com/ymgyt/syndicationd/commit/c311417bb69d22c7826d4ec931ec8dfe59042ca1))

## [v0.1.1] - 2024-02-12

### Features

- Add o11y crate by [@ymgyt](https://github.com/ymgyt) ([0a50517e](https://github.com/ymgyt/syndicationd/commit/0a50517e0b861973fac95ad5dba6f2c4d5b7270d))
- Add opentelemetry-tracing log bridge layer by [@ymgyt](https://github.com/ymgyt) ([92f22b56](https://github.com/ymgyt/syndicationd/commit/92f22b564357a0d43f8631212cf976338eb05a04))
- Add baggage propagation by [@ymgyt](https://github.com/ymgyt) ([d02e514c](https://github.com/ymgyt/syndicationd/commit/d02e514c8f6e32aa748c10dadb204153cba21ecc))
- Add opentelemetry layers by [@ymgyt](https://github.com/ymgyt) ([4d3f5bf3](https://github.com/ymgyt/syndicationd/commit/4d3f5bf3f45f31cfd014dbdf37a41a31ea0472ca))

### Miscellaneous Tasks

- Configure release flow by [@ymgyt](https://github.com/ymgyt) ([855d1063](https://github.com/ymgyt/syndicationd/commit/855d1063f5b476433fe0a7ab352b72d63a749e2e))
- Use hyphen as package name instead of underscore by [@ymgyt](https://github.com/ymgyt) ([0a8ed059](https://github.com/ymgyt/syndicationd/commit/0a8ed05997790f9f05c932c92fa2b2b2d74065a9))

### Testing

- Fix wrong assertion by [@ymgyt](https://github.com/ymgyt) ([392df0de](https://github.com/ymgyt/syndicationd/commit/392df0de6c4b7e8c34ae2d7c2f8ec764c23145f0))

<!-- generated by git-cliff -->
