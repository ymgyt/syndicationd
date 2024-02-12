# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

### Features

- Add baggage propagation by [@ymgyt](https://github.com/ymgyt) ([d02e514c](https://github.com/ymgyt/syndicationd/commit/d02e514c8f6e32aa748c10dadb204153cba21ecc))
- Add opentelemetry layers by [@ymgyt](https://github.com/ymgyt) ([4d3f5bf3](https://github.com/ymgyt/syndicationd/commit/4d3f5bf3f45f31cfd014dbdf37a41a31ea0472ca))
- Update ratatui to 0.26 and fix breaking apis by [@ymgyt](https://github.com/ymgyt) ([c482683a](https://github.com/ymgyt/syndicationd/commit/c482683a0083baf93a60ef31b280c49ac4eafccb))
- Change local time format by [@ymgyt](https://github.com/ymgyt) ([fb826165](https://github.com/ymgyt/syndicationd/commit/fb826165367eb97c0bec216db286bf1ee13fba07))
- Use tailwind color palettes by [@ymgyt](https://github.com/ymgyt) ([a93b8ec7](https://github.com/ymgyt/syndicationd/commit/a93b8ec753d3f0da2c4915cc258b3b1054ccef57))
- Change entries table constraint by [@ymgyt](https://github.com/ymgyt) ([53027a59](https://github.com/ymgyt/syndicationd/commit/53027a59aa1bb8c24deeb5696dac52f2704104bc))
- Add palette flag by [@ymgyt](https://github.com/ymgyt) ([04dc486d](https://github.com/ymgyt/syndicationd/commit/04dc486d0ab3043e021e164e70f5fe081e3c464d))
- Add in_flight by [@ymgyt](https://github.com/ymgyt) ([eae48336](https://github.com/ymgyt/syndicationd/commit/eae48336cc6e5298bc6c78599fa3054a134a170e))
- Add in flight throbber by [@ymgyt](https://github.com/ymgyt) ([fef77519](https://github.com/ymgyt/syndicationd/commit/fef77519e2ca59e5d267d6cecab8c008e92adc2c))
- Add instrument by [@ymgyt](https://github.com/ymgyt) ([dfbe9350](https://github.com/ymgyt/syndicationd/commit/dfbe93501542ff75361ddf3b158e21f7e77329b3))
- Impl kvsd client by [@ymgyt](https://github.com/ymgyt) ([6ae6de7a](https://github.com/ymgyt/syndicationd/commit/6ae6de7a2e783417b1a8d5d3c2b450109d83725f))
- Improve subscription input handling by [@ymgyt](https://github.com/ymgyt) ([309d8fac](https://github.com/ymgyt/syndicationd/commit/309d8fac0ea33438af61df374f32a73e235ec63f))
- Improve feed subscription flow by [@ymgyt](https://github.com/ymgyt) ([088d18df](https://github.com/ymgyt/syndicationd/commit/088d18df15486d4635a5dc2014f62b9fce6a9db6))
- Swap terminal restore step by [@ymgyt](https://github.com/ymgyt) ([2f9f2cb7](https://github.com/ymgyt/syndicationd/commit/2f9f2cb7830d7cb473b847f1969c9125428e4a6e))
- Remove unsubscribed entries by [@ymgyt](https://github.com/ymgyt) ([d29ba92e](https://github.com/ymgyt/syndicationd/commit/d29ba92e929d9d1348fa114ac2bdf210b76c5a1b))
- Reload entries when subscribe feed by [@ymgyt](https://github.com/ymgyt) ([6e0aa72b](https://github.com/ymgyt/syndicationd/commit/6e0aa72b67a17e7139b532940c24f70a7642a39d))
- Serve https by [@ymgyt](https://github.com/ymgyt) ([fbb9011e](https://github.com/ymgyt/syndicationd/commit/fbb9011e86acf6e4cf30f37a74e67d3202bbc5a0))
- Support axum_server graceful shutdown by [@ymgyt](https://github.com/ymgyt) ([880b6d3e](https://github.com/ymgyt/syndicationd/commit/880b6d3e8d0f90b711a1d6e8e1bf6fb1808e5161))
- Use cow by [@ymgyt](https://github.com/ymgyt) ([ab6ae298](https://github.com/ymgyt/syndicationd/commit/ab6ae298abeda1d7d3c67939bc70f0d2269e8654))
- Update default endpoint by [@ymgyt](https://github.com/ymgyt) ([e684b0cc](https://github.com/ymgyt/syndicationd/commit/e684b0cc4122a3fd4ece6a1e3697f71aaa311daf))

### Bug Fixes

- Workarround scrollbar rendering bug by [@ymgyt](https://github.com/ymgyt) ([d2982cb6](https://github.com/ymgyt/syndicationd/commit/d2982cb6c8fa385655290d953aa9243d3470382d))
- Build by [@ymgyt](https://github.com/ymgyt) ([bd340e9d](https://github.com/ymgyt/syndicationd/commit/bd340e9d30f101c891f53b2d2be10a0cf8833f4b))

### Miscellaneous Tasks

- Format toml by [@ymgyt](https://github.com/ymgyt) ([36677745](https://github.com/ymgyt/syndicationd/commit/3667774506106fe0f38d77efac9f4b27c70090aa))
- Update CHANGELOG by [@ymgyt](https://github.com/ymgyt) ([f5091f3c](https://github.com/ymgyt/syndicationd/commit/f5091f3ceff04b9ff818bb4e0ce0e4bbe9851177))
- Update CHANGELOG by [@ymgyt](https://github.com/ymgyt) ([99f036df](https://github.com/ymgyt/syndicationd/commit/99f036dfe227c1670f967aa949116e3ae8a2c97b))
- Use hyphen as package name instead of underscore by [@ymgyt](https://github.com/ymgyt) ([0a8ed059](https://github.com/ymgyt/syndicationd/commit/0a8ed05997790f9f05c932c92fa2b2b2d74065a9))
- Update CHANGELOG by [@ymgyt](https://github.com/ymgyt) ([e688670c](https://github.com/ymgyt/syndicationd/commit/e688670c853718a1cb825cb787861dffe55046d1))
- Rename synd-authn to synt-auth to publish as a new crate by [@ymgyt](https://github.com/ymgyt) ([59ae4ffa](https://github.com/ymgyt/syndicationd/commit/59ae4ffa51f5323fa4a3aae5e30e950b15730519))
- Update CHANGELOG by [@ymgyt](https://github.com/ymgyt) ([31eb8a34](https://github.com/ymgyt/syndicationd/commit/31eb8a3472e770931fab427e2a8c74a9754b157a))
- Update CHANGELOG by [@ymgyt](https://github.com/ymgyt) ([e1910ce1](https://github.com/ymgyt/syndicationd/commit/e1910ce120ca9d8f38fa4c479156f723d54ae59c))

### Refactor

- Rename crates by [@ymgyt](https://github.com/ymgyt) ([ce0982e4](https://github.com/ymgyt/syndicationd/commit/ce0982e497647b23dcf07e39d525121bcd9ac1fa))
- Create synd_authn crate by [@ymgyt](https://github.com/ymgyt) ([682bcc6f](https://github.com/ymgyt/syndicationd/commit/682bcc6ff3c035be566dea99d2487e0173537c8d))
- Use clippy pedantic by [@ymgyt](https://github.com/ymgyt) ([328ddade](https://github.com/ymgyt/syndicationd/commit/328ddadebbad5381271c5e84cce2d6888252e70c))
- Clippy by [@ymgyt](https://github.com/ymgyt) ([a1693b36](https://github.com/ymgyt/syndicationd/commit/a1693b36b73ad3987af9a853e214392d8b1eae8d))
- Fix lint by [@ymgyt](https://github.com/ymgyt) ([aac00b98](https://github.com/ymgyt/syndicationd/commit/aac00b98335bb75cc57fdea0875bfd675bf8f3cc))
- Rename tab by [@ymgyt](https://github.com/ymgyt) ([be4add1e](https://github.com/ymgyt/syndicationd/commit/be4add1e261c505d87b174795274236fd8ce46e7))

### Testing

- Impl device flow test case by [@ymgyt](https://github.com/ymgyt) ([93572902](https://github.com/ymgyt/syndicationd/commit/9357290265a4fbf8d78721e4f9f1904b1cf5b12a))
- Add auth flow case by [@ymgyt](https://github.com/ymgyt) ([6d2b1905](https://github.com/ymgyt/syndicationd/commit/6d2b1905d9b06bd9ed670f210cd590f89405c37c))
- Run kvsd in test by [@ymgyt](https://github.com/ymgyt) ([923e65a1](https://github.com/ymgyt/syndicationd/commit/923e65a131bed1a0a10d073b0eb9d5091cc184fe))
- Run integration test by [@ymgyt](https://github.com/ymgyt) ([20c0bc2d](https://github.com/ymgyt/syndicationd/commit/20c0bc2d31a938d3103fafedba5a10b4a9bba9ae))
- Fix tls conf path by [@ymgyt](https://github.com/ymgyt) ([e3d764a4](https://github.com/ymgyt/syndicationd/commit/e3d764a453b527a98b1eaf268ead67469c0e192d))

<!-- generated by git-cliff -->
