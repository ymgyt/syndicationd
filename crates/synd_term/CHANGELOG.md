# Changelog

All notable changes to this project will be documented in this file.

## [v0.1.10] 2024-03-19

### Features

- Make the order of keymap help consistent by [@ymgyt](https://github.com/ymgyt) ([76d385e3](https://github.com/ymgyt/syndicationd/commit/76d385e31d3f46513b4c5c39e6166f72874f16be))
- Change time format delimiter by [@ymgyt](https://github.com/ymgyt) ([b0768bc1](https://github.com/ymgyt/syndicationd/commit/b0768bc11980f9a620b6857639a33a0c01cc3fe8))
- Change entry column name by [@ymgyt](https://github.com/ymgyt) ([fd15bf63](https://github.com/ymgyt/syndicationd/commit/fd15bf633dab04159b69754901b6962cdc3f6d38))

### Miscellaneous Tasks

- Set log level for the credential restore process to debug by [@ymgyt](https://github.com/ymgyt) ([30dc7a2f](https://github.com/ymgyt/syndicationd/commit/30dc7a2f71e34ce431435acdc21c170c86c677b0))

## [v0.1.9] - 2024-03-18

### Features

- Fallback latest entries published date by [@ymgyt](https://github.com/ymgyt) ([98b57a10](https://github.com/ymgyt/syndicationd/commit/98b57a108e9b137b47e76f9e88bfa900db46ff8a))
- Handle ctrl-c by [@ymgyt](https://github.com/ymgyt) ([ef2842e2](https://github.com/ymgyt/syndicationd/commit/ef2842e2cdd1bf4e0468e69e5cdea06869fa17b3))
- Make the space policy consistent by [@ymgyt](https://github.com/ymgyt) ([0a3d9dfb](https://github.com/ymgyt/syndicationd/commit/0a3d9dfb8c91f8ea2875dd99a423ae3189f17e56))
- Change detail border type by [@ymgyt](https://github.com/ymgyt) ([099c8524](https://github.com/ymgyt/syndicationd/commit/099c8524fa8a2af1fefd1bab3f8cd6cb91edee42))
- Change feed meta widget from list to table by [@ymgyt](https://github.com/ymgyt) ([f583e2f2](https://github.com/ymgyt/syndicationd/commit/f583e2f27dbe11c0f6348ab43bd918877d719d6e))

### Bug Fixes

- Remove debug logging by [@ymgyt](https://github.com/ymgyt) ([066b3cc0](https://github.com/ymgyt/syndicationd/commit/066b3cc04490d15618a1503098fe4e1aae411198))

### Refactor

- Rename jwt_decoder to jwt_service by [@ymgyt](https://github.com/ymgyt) ([fa6f178c](https://github.com/ymgyt/syndicationd/commit/fa6f178cbad30c6100cde0a9c77ca2eed1eadb52))

## [v0.1.8] - 2024-03-17

### Features

- Use nerd fond in prompt by [@ymgyt](https://github.com/ymgyt) ([b864e277](https://github.com/ymgyt/syndicationd/commit/b864e27793087d12ea63a5215df509c25854ac46))
- Use nerd font in columns by [@ymgyt](https://github.com/ymgyt) ([278fbbe8](https://github.com/ymgyt/syndicationd/commit/278fbbe833abd770d25c41f7e9e4267514ba2714))
- Support google login by [@ymgyt](https://github.com/ymgyt) ([a55c3109](https://github.com/ymgyt/syndicationd/commit/a55c31094a723e6541300898b8dab875b11a6f4a))
- Error if google jwt email is not verified by [@ymgyt](https://github.com/ymgyt) ([a8ee97d0](https://github.com/ymgyt/syndicationd/commit/a8ee97d05714f9ac9f54df61ddfc8ea1bca2cea8))
- Add nerd font to feed detail component by [@ymgyt](https://github.com/ymgyt) ([1379a297](https://github.com/ymgyt/syndicationd/commit/1379a297d6129675225f16c578aa51930293cda9))

### Refactor

- Rename device flow poll method by [@ymgyt](https://github.com/ymgyt) ([19fe8c4f](https://github.com/ymgyt/syndicationd/commit/19fe8c4fbce84cc8dab6678d38653fa304bd26ff))

## [v0.1.7] - 2024-03-12

### Features

- Show first graphql error in ui by [@ymgyt](https://github.com/ymgyt) ([ca29ea02](https://github.com/ymgyt/syndicationd/commit/ca29ea021937f9818555dd64659041da04762f15))
- Add export command by [@ymgyt](https://github.com/ymgyt) ([9bb73182](https://github.com/ymgyt/syndicationd/commit/9bb731820e1f064f1d5776f5285ea57785596006))
- Print export json schema by [@ymgyt](https://github.com/ymgyt) ([3023c3f7](https://github.com/ymgyt/syndicationd/commit/3023c3f74824d4001b7d684b53b3e4e978384302))

### Miscellaneous Tasks

- Configure oranda changelog by [@ymgyt](https://github.com/ymgyt) ([1aecb8ce](https://github.com/ymgyt/syndicationd/commit/1aecb8ce5a31b766d63d8bb283e993508e379608))
- Specify features to build by [@ymgyt](https://github.com/ymgyt) ([b7db28d1](https://github.com/ymgyt/syndicationd/commit/b7db28d1eb796899a48cf23366499e287fe775fa))

## [v0.1.6] - 2024-02-28

### Features

- Add move to first/last commnad by [@ymgyt](https://github.com/ymgyt) ([4bc7f482](https://github.com/ymgyt/syndicationd/commit/4bc7f482d10e52339057784052194d8ddeff30b5))

### Miscellaneous Tasks

- Read changelog by [@ymgyt](https://github.com/ymgyt) ([9095f676](https://github.com/ymgyt/syndicationd/commit/9095f6764cf8ee1bf2acff85f4df4250bb0e4167))

## [v0.1.5] - 2024-02-25

### Features

- Use env var as default flag value by [@ymgyt](https://github.com/ymgyt) ([c7887e92](https://github.com/ymgyt/syndicationd/commit/c7887e925d6856f761051d118662d77d35d08968))
- Handle subscribe feed error by [@ymgyt](https://github.com/ymgyt) ([d6abb26e](https://github.com/ymgyt/syndicationd/commit/d6abb26eb7ea75ba479f07cb83ff680a1708c6af))
- Use entry updated if published is none by [@ymgyt](https://github.com/ymgyt) ([2b16b51c](https://github.com/ymgyt/syndicationd/commit/2b16b51c3cadb7b0dd74a848ae43ff078372b678))
- Add feed detail widget by [@ymgyt](https://github.com/ymgyt) ([836258d4](https://github.com/ymgyt/syndicationd/commit/836258d490bd63de7bc481bc6ad9f5866f5e861e))
- Add reload by [@ymgyt](https://github.com/ymgyt) ([de11397c](https://github.com/ymgyt/syndicationd/commit/de11397cde2d003b81eb029752673f214724c4c2))
- Add client timeout flag by [@ymgyt](https://github.com/ymgyt) ([86f5bf43](https://github.com/ymgyt/syndicationd/commit/86f5bf437193791905d82b441f9bc39cecc401ff))
- Add check command by [@ymgyt](https://github.com/ymgyt) ([018c0c22](https://github.com/ymgyt/syndicationd/commit/018c0c222704746315e3a0faf852a0868f719a00))

### Bug Fixes

- Use name instead of bin_name by [@ymgyt](https://github.com/ymgyt) ([1a9b81dd](https://github.com/ymgyt/syndicationd/commit/1a9b81dd6a9734ea99d63bac052b73b55e9470fd))

### Documentation

- Update install description by [@ymgyt](https://github.com/ymgyt) ([13ecd094](https://github.com/ymgyt/syndicationd/commit/13ecd094ae813517d7554c54572dcc2a83654311))
- Configure oranda by [@ymgyt](https://github.com/ymgyt) ([91e158df](https://github.com/ymgyt/syndicationd/commit/91e158df904e91a27d8f68217500ad76ea91ffe9))
- Configure oranda social by [@ymgyt](https://github.com/ymgyt) ([1624d62a](https://github.com/ymgyt/syndicationd/commit/1624d62a51fdeea38594869c707d036c792f2e61))
- Configure oranda components by [@ymgyt](https://github.com/ymgyt) ([3dcbc57a](https://github.com/ymgyt/syndicationd/commit/3dcbc57a435321d7f39e7e39bf90b44b1b712e7b))

### Miscellaneous Tasks

- Set clap bin_name by [@ymgyt](https://github.com/ymgyt) ([dca2b898](https://github.com/ymgyt/syndicationd/commit/dca2b898b2cd596b0655797e037c5a5c82cf9b0a))
- Configure feed detail height by [@ymgyt](https://github.com/ymgyt) ([3de1c98f](https://github.com/ymgyt/syndicationd/commit/3de1c98fdce08a622323c269373d2ece0b00ec74))
- Typo by [@ymgyt](https://github.com/ymgyt) ([13ccdb5d](https://github.com/ymgyt/syndicationd/commit/13ccdb5d7c80627913d9858887b7b6d84dc07dff))
- Trim prefix from changelog by [@ymgyt](https://github.com/ymgyt) ([95d44877](https://github.com/ymgyt/syndicationd/commit/95d448773ec7ab009fbece0928854364679b6f2c))
- Set brew fomula name by [@ymgyt](https://github.com/ymgyt) ([8b33da9a](https://github.com/ymgyt/syndicationd/commit/8b33da9afc98ab6cdc12a0ca48829b27f39c63f6))
- Change default endpoint by [@ymgyt](https://github.com/ymgyt) ([c352b871](https://github.com/ymgyt/syndicationd/commit/c352b8713f4acbaf022c857e036d33fc688c9991))
- Add homepage to package metadata by [@ymgyt](https://github.com/ymgyt) ([4bfdb49e](https://github.com/ymgyt/syndicationd/commit/4bfdb49e317e18ff6345ce1b8e8071f0497a1a5f))
- Use workspace dep by [@ymgyt](https://github.com/ymgyt) ([92163422](https://github.com/ymgyt/syndicationd/commit/921634227a53e2a3594d1cedb5116e53dc43baa4))
- Enable cargo-dist explicitly by [@ymgyt](https://github.com/ymgyt) ([3a04e732](https://github.com/ymgyt/syndicationd/commit/3a04e7327a752dea0497f900f0a96364977de96e))

### Testing

- Use tempfile instead of deprecated tempdir by [@ymgyt](https://github.com/ymgyt) ([749de1db](https://github.com/ymgyt/syndicationd/commit/749de1dba0235e30e1e79ca10849d049005c0a15))

## [v0.1.3] - 2024-02-19

### Features

- Improve feed url parse by [@ymgyt](https://github.com/ymgyt) ([460e87d0](https://github.com/ymgyt/syndicationd/commit/460e87d00f9acd83a922a97c339a620c0037c14f))

## [v0.1.2] - 2024-02-19

### Features

- Change log directive env key by [@ymgyt](https://github.com/ymgyt) ([6da13de1](https://github.com/ymgyt/syndicationd/commit/6da13de165e3ec57e3c15c08dc6f8237debe082e))

### Miscellaneous Tasks

- Use include by [@ymgyt](https://github.com/ymgyt) ([cdff7a76](https://github.com/ymgyt/syndicationd/commit/cdff7a762d6cca85160f01b6f646d8baa6e47e59))

### Refactor

- Remove comment by [@ymgyt](https://github.com/ymgyt) ([2b042c69](https://github.com/ymgyt/syndicationd/commit/2b042c696d5c64c3dd2608e4382f50e4a76ed38c))

## [v0.1.1] - 2024-02-12

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
