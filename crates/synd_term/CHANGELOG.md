# Changelog

All notable changes to this project will be documented in this file.

## [v0.3.0] 2024-07-11

### Features

- Handle github secondary rate limit error by [@ymgyt](https://github.com/ymgyt) ([9f690c67](https://github.com/ymgyt/syndicationd/commit/9f690c675870b359150738c66029224f5c94b52f))
- Limit the number of concurrent executions of job futures by [@ymgyt](https://github.com/ymgyt) ([62b51bb0](https://github.com/ymgyt/syndicationd/commit/62b51bb041b151593cc634ca30114d356243ceff))

### Refactor

- Use the filter of FilterableVec as the primary source by [@ymgyt](https://github.com/ymgyt) ([f1739fe8](https://github.com/ymgyt/syndicationd/commit/f1739fe811749466038070d242a2913a7524baaf))
- Use macro to impl newtype by [@ymgyt](https://github.com/ymgyt) ([17797a5c](https://github.com/ymgyt/syndicationd/commit/17797a5c25b2c30d4b20dfede7cf331fd1ad8c6b))
- Split background futures to another jobs by [@ymgyt](https://github.com/ymgyt) ([995f143b](https://github.com/ymgyt/syndicationd/commit/995f143bce4237bfceacad816b86522e59d268a2))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.3.0-rc.2...


## [v0.3.0-rc.2] - 2024-07-07

### Bug Fixes

- Apply initial filter by [@ymgyt](https://github.com/ymgyt) ([47ed7abf](https://github.com/ymgyt/syndicationd/commit/47ed7abf59ca8d11abff337dc1e141b83752595e))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.3.0-rc.1...synd-term-v0.3.0-rc.2


## [v0.3.0-rc.1] - 2024-07-07

### Features

- Use local timezone when displaying time (#71) by [@ymgyt](https://github.com/ymgyt) ([36212b4b](https://github.com/ymgyt/syndicationd/commit/36212b4b1d00855b0b4206a45714cc42359dfb8e))
- Handle terminal focus event by [@ymgyt](https://github.com/ymgyt) ([27f02a36](https://github.com/ymgyt/syndicationd/commit/27f02a36aead07994fb495eb4305d32e3fd0bdd4))
- Support github notification (#77) by [@ymgyt](https://github.com/ymgyt) ([b3fc9958](https://github.com/ymgyt/syndicationd/commit/b3fc9958e739df57fb212ca0f986cb9cc25af333))
- Category filtering for github notifications (#78) by [@ymgyt](https://github.com/ymgyt) ([e67b0502](https://github.com/ymgyt/syndicationd/commit/e67b0502e8039844f2dc579af84f9caa9ede8bfe))
- Display labels for github issues and PRs by [@ymgyt](https://github.com/ymgyt) ([1cd28d0c](https://github.com/ymgyt/syndicationd/commit/1cd28d0c06305d44820e19ecf4ae9bc08a54c1c0))
- Add github filter conditions (#85) by [@ymgyt](https://github.com/ymgyt) ([a1135c7e](https://github.com/ymgyt/syndicationd/commit/a1135c7e94f71b74cbb98b6ce3e26c67d1c029f4))
- Add mark_as_done_all command by [@ymgyt](https://github.com/ymgyt) ([4633d73c](https://github.com/ymgyt/syndicationd/commit/4633d73caaef414cc830d445b1af4431763ac389))
- Persist github notifications filter options by [@ymgyt](https://github.com/ymgyt) ([28ba85ee](https://github.com/ymgyt/syndicationd/commit/28ba85ee2957e3a513b8814e46fb15b741f0e67a))
- Handle github unauthorized error by [@ymgyt](https://github.com/ymgyt) ([0f9acbb5](https://github.com/ymgyt/syndicationd/commit/0f9acbb536c1241534f8a67306a589db819bf638))

### Bug Fixes

- Make tab width dynamic by [@ymgyt](https://github.com/ymgyt) ([9679d7da](https://github.com/ymgyt/syndicationd/commit/9679d7da3e6315bebb46a70a13a3ab4e8ce24fa5))
- Fix graphql schema path by [@ymgyt](https://github.com/ymgyt) ([f1abe4b7](https://github.com/ymgyt/syndicationd/commit/f1abe4b76294c4a6111fa440c3c77fd3e55c5d23))

### Miscellaneous Tasks

- Update ratatui from 0.26.3 to 0.27.0 by [@ymgyt](https://github.com/ymgyt) ([968c3256](https://github.com/ymgyt/syndicationd/commit/968c32564b6dd2882da413d88320077989464f8c))
- Include graphql files by [@ymgyt](https://github.com/ymgyt) ([453d630e](https://github.com/ymgyt/syndicationd/commit/453d630e07b5f849b0f981619e0403977b1887c3))
- Copy github graphql schema by [@ymgyt](https://github.com/ymgyt) ([ce90f72b](https://github.com/ymgyt/syndicationd/commit/ce90f72b9acf461d6aeadfe1413602dcc851a2ef))

### Styling

- Use italic modifier for gh notification filters by [@ymgyt](https://github.com/ymgyt) ([ead785ac](https://github.com/ymgyt/syndicationd/commit/ead785acfbb32d766f238b847b87b0a4a8ed6654))

### Testing

- Add test case for focus gained event by [@ymgyt](https://github.com/ymgyt) ([423d201f](https://github.com/ymgyt/syndicationd/commit/423d201f35b94217c0ff9a68e1442f261dcfb2e9))
- Add gql fixtures to gh notifications test by [@ymgyt](https://github.com/ymgyt) ([1616c526](https://github.com/ymgyt/syndicationd/commit/1616c526e076547648661705fec2fafd66090e88))
- Add test case for filtering gh notifications by [@ymgyt](https://github.com/ymgyt) ([cf45254d](https://github.com/ymgyt/syndicationd/commit/cf45254d7db5080cf1753bb33cfa2c9d91c10cce))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.2.6...synd-term-v0.3.0-rc.1


## [v0.2.6] - 2024-06-18

### Features

- Simplify ui by [@ymgyt](https://github.com/ymgyt) ([b2fa928d](https://github.com/ymgyt/syndicationd/commit/b2fa928de37cf0c431d63e2e2f2b17e6dee19250))
- Show entry detail by [@ymgyt](https://github.com/ymgyt) ([e9162afa](https://github.com/ymgyt/syndicationd/commit/e9162afa903277d751ed8abf964a275668f6096c))
- Inform latest release by [@ymgyt](https://github.com/ymgyt) ([a65eb66c](https://github.com/ymgyt/syndicationd/commit/a65eb66ccedd737f95dcea78c0e12770016ffade))
- Match feed url by [@ymgyt](https://github.com/ymgyt) ([d077a320](https://github.com/ymgyt/syndicationd/commit/d077a320226e26a7cba928c619822c8509b7548b))

### Documentation

- Use oranda mdbook component for rendering privacy policy by [@ymgyt](https://github.com/ymgyt) ([c91bc08c](https://github.com/ymgyt/syndicationd/commit/c91bc08ce4cf92629ca3438be0dabe5f97263fa9))

### Miscellaneous Tasks

- Update ratatui from 0.26.1 to 0.26.3 by [@ymgyt](https://github.com/ymgyt) ([5260fc8b](https://github.com/ymgyt/syndicationd/commit/5260fc8b18dcd268ec2f912f9d7ac88e7de214fd))
- Simplyfi fileter ui by [@ymgyt](https://github.com/ymgyt) ([e7815ada](https://github.com/ymgyt/syndicationd/commit/e7815adaeff96171e9f6ab825591dc12794d08a2))
- Rename homebrew formula from synd to synd-term by [@ymgyt](https://github.com/ymgyt) ([79ca0f87](https://github.com/ymgyt/syndicationd/commit/79ca0f87bb69b9aa9e093a41c5667251f86c56fc))
- Add docker description by [@ymgyt](https://github.com/ymgyt) ([a7ab8ae2](https://github.com/ymgyt/syndicationd/commit/a7ab8ae22f71b955def42a9b306b6ee1a42c8d9e))
- Rename oranda project from synd to synd-term by [@ymgyt](https://github.com/ymgyt) ([383a3592](https://github.com/ymgyt/syndicationd/commit/383a3592d764482e11f3c65b3afaea607d2b9acb))
- Use default-features instead of default_features by [@ymgyt](https://github.com/ymgyt) ([fd827f4c](https://github.com/ymgyt/syndicationd/commit/fd827f4cba9e576a9342e1115fdf7e804471d438))

### Refactor

- Remove unused code by [@ymgyt](https://github.com/ymgyt) ([bd428f17](https://github.com/ymgyt/syndicationd/commit/bd428f1724e2e97b6fbd97dad4d1e539b2e9420d))
- Avoid rendering during key event handling if possible by [@ymgyt](https://github.com/ymgyt) ([db42f5c7](https://github.com/ymgyt/syndicationd/commit/db42f5c7236495bdde39160774e8286d809a844b))
- Abstract cache access by [@ymgyt](https://github.com/ymgyt) ([32ae36e0](https://github.com/ymgyt/syndicationd/commit/32ae36e05eafcb649df3c4fcdd9dac4f33272e9c))
- Use ApplicationBuilder to construct Application by [@ymgyt](https://github.com/ymgyt) ([985edfd9](https://github.com/ymgyt/syndicationd/commit/985edfd9304e748f9f21a93b6f771b652a9ea67a))
- Use tokio_util CancellationToken instead of mpsc channel by [@ymgyt](https://github.com/ymgyt) ([3ca15bf8](https://github.com/ymgyt/syndicationd/commit/3ca15bf854882d64b51b825bbeff6db6f2c66ae0))
- Remove unused code by [@ymgyt](https://github.com/ymgyt) ([a46f99fd](https://github.com/ymgyt/syndicationd/commit/a46f99fdda7d3eed9a847bf53f2add980884d699))
- Move generated gql client code to generated dir by [@ymgyt](https://github.com/ymgyt) ([8600f559](https://github.com/ymgyt/syndicationd/commit/8600f559909ba39da7881d0f6d68dfab664fb7d4))
- Remove duplicate processing in InFlight::remove by [@ymgyt](https://github.com/ymgyt) ([0b9c8b0c](https://github.com/ymgyt/syndicationd/commit/0b9c8b0c4c40f00eccbe6592eafdd9be04b9fa4c))

### Styling

- Remove right padding for consistency by [@ymgyt](https://github.com/ymgyt) ([79b59ca5](https://github.com/ymgyt/syndicationd/commit/79b59ca520c11041f1b68dd146751d2c91a3de32))
- Remove icon from table header for alignment by [@ymgyt](https://github.com/ymgyt) ([ef156c94](https://github.com/ymgyt/syndicationd/commit/ef156c9407a219b4fbe47dca9c067773153a4614))
- Fix search alignment by [@ymgyt](https://github.com/ymgyt) ([43f30fc6](https://github.com/ymgyt/syndicationd/commit/43f30fc6c7848e581af766320162abbfd7800ead))

### Testing

- Use insta for buffer assersions by [@ymgyt](https://github.com/ymgyt) ([72f90cb3](https://github.com/ymgyt/syndicationd/commit/72f90cb347422a464c3cd29ce27586107544871f))
- Add command test by [@ymgyt](https://github.com/ymgyt) ([b07b0707](https://github.com/ymgyt/syndicationd/commit/b07b07074308f234feb4444e6d0d2252c56befcf))
- Setup application in helper method by [@ymgyt](https://github.com/ymgyt) ([489bd75f](https://github.com/ymgyt/syndicationd/commit/489bd75f29f348cf60fb61468dbb7c8ab2844663))
- Filtered out non-test-related events during integration by [@ymgyt](https://github.com/ymgyt) ([eafc1e4f](https://github.com/ymgyt/syndicationd/commit/eafc1e4fadbe4cdd0c5198c08754e2dbdb469e54))
- Add subscribe integration test case by [@ymgyt](https://github.com/ymgyt) ([8d9a30db](https://github.com/ymgyt/syndicationd/commit/8d9a30dbf0ed713c4efc0c1c9718465a848c6980))
- Add command test by [@ymgyt](https://github.com/ymgyt) ([53a16c71](https://github.com/ymgyt/syndicationd/commit/53a16c715705d1d2968c7a9d12033837b8451cc7))
- Add google authentication test case by [@ymgyt](https://github.com/ymgyt) ([7590f197](https://github.com/ymgyt/syndicationd/commit/7590f197e1aea294b04f56b6fd370e8e6086f1c9))
- Add fetch entries fixture test by [@ymgyt](https://github.com/ymgyt) ([4f3b8e23](https://github.com/ymgyt/syndicationd/commit/4f3b8e232085cf904fd02d6105bd8bf6eb63bcc1))
- Add filter entries integration test case by [@ymgyt](https://github.com/ymgyt) ([d005d0d2](https://github.com/ymgyt/syndicationd/commit/d005d0d2822960ae12190b822b3c1e4a16aa4c29))
- Handle not tty case by [@ymgyt](https://github.com/ymgyt) ([b3ada7c0](https://github.com/ymgyt/syndicationd/commit/b3ada7c068cbb04230715e1764535768715a6613))
- Add pperiodic refresher test case by [@ymgyt](https://github.com/ymgyt) ([6e9a19da](https://github.com/ymgyt/syndicationd/commit/6e9a19da92dfe5d006756d19d8c4ed1bdd9690c5))
- Add refreshing expired jwt test case (#53) by [@ymgyt](https://github.com/ymgyt) ([ffd73907](https://github.com/ymgyt/syndicationd/commit/ffd73907b88aab2c044362c0debdfa9b012571bd))
- Add test to direction by [@ymgyt](https://github.com/ymgyt) ([90f8492a](https://github.com/ymgyt/syndicationd/commit/90f8492a7450f6cc2b4a025d451a9d15144e0ee5))
- Add test case that resize terminal by [@ymgyt](https://github.com/ymgyt) ([05251ee4](https://github.com/ymgyt/syndicationd/commit/05251ee4fbfc776c0acaa0980e8c44b84038fa28))
- Add test case that edit and open feed by [@ymgyt](https://github.com/ymgyt) ([a2e99d98](https://github.com/ymgyt/syndicationd/commit/a2e99d98d2fed6497ba783aeba273218a1c21cfc))
- Add error handling test case by [@ymgyt](https://github.com/ymgyt) ([25641b2c](https://github.com/ymgyt/syndicationd/commit/25641b2c4a4aff371254531f0104cf477a24fc2b))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.2.5...synd-term-v0.2.6


## [v0.2.5] - 2024-05-23

### Features

- Add ferra, solarized_dark, helix themes by [@ymgyt](https://github.com/ymgyt) ([d463de09](https://github.com/ymgyt/syndicationd/commit/d463de090b91d792aed28d3d4a1e423989281a4c))

### Miscellaneous Tasks

- Avoid using fonts that cause issues when terminal opacity is enabled by [@ymgyt](https://github.com/ymgyt) ([13c7b8d5](https://github.com/ymgyt/syndicationd/commit/13c7b8d506cecad3f255f22809eacdee2419db2a))

### Refactor

- Use std::ops::ControlFlow for app loop control by [@ymgyt](https://github.com/ymgyt) ([99423986](https://github.com/ymgyt/syndicationd/commit/9942398608c174c1ff41b7c18f1e3169fc857c7d))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.2.4...synd-term-v0.2.5


## [v0.2.4] - 2024-05-06

### Features

- Show big text on login by [@ymgyt](https://github.com/ymgyt) ([d4a5b18e](https://github.com/ymgyt/syndicationd/commit/d4a5b18e7d9771a4ff5647da059f187ce0c240b6))
- Refresh google id token periodically by [@ymgyt](https://github.com/ymgyt) ([b5e0ae1f](https://github.com/ymgyt/syndicationd/commit/b5e0ae1f22f0a4c14479fe55caf11c4d4d0e6a22))
- Friendly nom parse error by [@ymgyt](https://github.com/ymgyt) ([8664e3d7](https://github.com/ymgyt/syndicationd/commit/8664e3d71ab21fd0b34515bef4efd6d9d595b11e))
- Paginate entries and feeds by [@ymgyt](https://github.com/ymgyt) ([794f65da](https://github.com/ymgyt/syndicationd/commit/794f65dabb114d7f80069b6d65813a39560ffc40))
- Make entries limit configurable by [@ymgyt](https://github.com/ymgyt) ([206bbad7](https://github.com/ymgyt/syndicationd/commit/206bbad791f5c4dc3800af8bfd190cc9ad1469e5))
- Show entries count indicator by [@ymgyt](https://github.com/ymgyt) ([fa4abc7e](https://github.com/ymgyt/syndicationd/commit/fa4abc7e0961844bede78595dbca06fd37dcbe28))
- Add unsubscribe popup by [@ymgyt](https://github.com/ymgyt) ([d7db5140](https://github.com/ymgyt/syndicationd/commit/d7db51402c940c4fce41bf9b2c9fd18b08aef25b))

### Bug Fixes

- Filter categories duplication by [@ymgyt](https://github.com/ymgyt) ([60ec0f7a](https://github.com/ymgyt/syndicationd/commit/60ec0f7a592519404bec74006db35059e73baae7))
- Handle too small width case by [@ymgyt](https://github.com/ymgyt) ([62b5b336](https://github.com/ymgyt/syndicationd/commit/62b5b3365b341432aaf0e5fc7cf1dc970e49646c))

### Miscellaneous Tasks

- Change feed entries count to fetch by [@ymgyt](https://github.com/ymgyt) ([979231e9](https://github.com/ymgyt/syndicationd/commit/979231e9761bc3b4a041648155018fd7077456d6))
- Prevent selection out of index by [@ymgyt](https://github.com/ymgyt) ([1cf01601](https://github.com/ymgyt/syndicationd/commit/1cf01601325b671e62ef4398d73e4aa61c9cffbc))
- Make column order consistent by [@ymgyt](https://github.com/ymgyt) ([fecafd98](https://github.com/ymgyt/syndicationd/commit/fecafd988b937d57a7a62cc8c1abc6dd903e4141))
- Logging feeds that failed to fetch by [@ymgyt](https://github.com/ymgyt) ([425548cb](https://github.com/ymgyt/syndicationd/commit/425548cbab0728ac54d28c30e5e76ba384e50c78))

### Refactor

- Clippy by [@ymgyt](https://github.com/ymgyt) ([ddc8fa66](https://github.com/ymgyt/syndicationd/commit/ddc8fa66d5d6d7b4dcb3892a147bf90552080cbf))
- Use bitflags to manage app flags by [@ymgyt](https://github.com/ymgyt) ([aa2d6c49](https://github.com/ymgyt/syndicationd/commit/aa2d6c491c591e4f966c87d2489395f6f96cf3fb))
- Count keymap capacity by [@ymgyt](https://github.com/ymgyt) ([466368f4](https://github.com/ymgyt/syndicationd/commit/466368f46b65b325959e740358d816fb9d602dd7))
- Rename parse module to service by [@ymgyt](https://github.com/ymgyt) ([256542d9](https://github.com/ymgyt/syndicationd/commit/256542d9955811eac0c26b350f528cce1106dd50))
- Reduce visibility by [@ymgyt](https://github.com/ymgyt) ([08df3e55](https://github.com/ymgyt/syndicationd/commit/08df3e55dd3deac1ef7f7445a2cedaa9b8d20bdb))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.2.3...synd-term-v0.2.4


## [v0.2.3] - 2024-04-29

### Features

- Add search by [@ymgyt](https://github.com/ymgyt) ([ad68a603](https://github.com/ymgyt/syndicationd/commit/ad68a603161f3ed0d0722eccb010851b82b6276e))

### Miscellaneous Tasks

- Change oranda project name from synd-term to synd by [@ymgyt](https://github.com/ymgyt) ([802892ad](https://github.com/ymgyt/syndicationd/commit/802892ad8351c546e5a80b6edeeba981a515a526))
- Rename clear command to clean by [@ymgyt](https://github.com/ymgyt) ([767adc34](https://github.com/ymgyt/syndicationd/commit/767adc34460a06dc8771fba55f7b2affd2da994c))

### Refactor

- Use FeedUrl instead of String by [@ymgyt](https://github.com/ymgyt) ([7503ae0e](https://github.com/ymgyt/syndicationd/commit/7503ae0e8c72061ce1f1bcb01112b55c744beac6))
- Make tests module consistent by [@ymgyt](https://github.com/ymgyt) ([a0c2c530](https://github.com/ymgyt/syndicationd/commit/a0c2c5300372f9a7d9e7f96c3a2bda5a620e755f))
- Rename prompt to status line by [@ymgyt](https://github.com/ymgyt) ([6e3c8850](https://github.com/ymgyt/syndicationd/commit/6e3c885057729f2e08c524224584c097f72a3f59))
- Rename filter method by [@ymgyt](https://github.com/ymgyt) ([4cc525fc](https://github.com/ymgyt/syndicationd/commit/4cc525fc6d5644783c9f93cbd60ffc65a0a8cb52))

### Testing

- Add matcher test by [@ymgyt](https://github.com/ymgyt) ([f1dc9564](https://github.com/ymgyt/syndicationd/commit/f1dc9564a371fee96b0b8a742eeb87cf8474397e))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.2.2...synd-term-v0.2.3


## [v0.2.2] - 2024-04-18

### Bug Fixes

- Use selected_feed to render feed detail by [@ymgyt](https://github.com/ymgyt) ([404cc4cf](https://github.com/ymgyt/syndicationd/commit/404cc4cf6f52129a9a32bb58a23a3e9eb1e98efb))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.2.1...synd-term-v0.2.2


## [v0.2.1] - 2024-04-17

### Bug Fixes

- Rollback ratatui from 0.26.2 to 0.26.1 by [@ymgyt](https://github.com/ymgyt) ([75b6db7f](https://github.com/ymgyt/syndicationd/commit/75b6db7ff5f237dba68fdb0480c1af4edede7dbd))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.2.0...synd-term-v0.2.1


## [v0.2.0] - 2024-04-17

### Features

- Support go to first/end keymap by [@ymgyt](https://github.com/ymgyt) ([874cfbef](https://github.com/ymgyt/syndicationd/commit/874cfbefca272136dfdafe15b97f10607f3112e7))
- Instrument graphql task monitor by [@ymgyt](https://github.com/ymgyt) ([cb44f3b8](https://github.com/ymgyt/syndicationd/commit/cb44f3b8af19eeecf16c500bd4478da28c5576ec))
- Parse feed category and requirement by [@ymgyt](https://github.com/ymgyt) ([17b62885](https://github.com/ymgyt/syndicationd/commit/17b628850eca335d7a6e7501c021d1f94d622a6d))
- Support feed annotations by [@ymgyt](https://github.com/ymgyt) ([937b561d](https://github.com/ymgyt/syndicationd/commit/937b561df1ae512da54408aa1996361cf9ca06ed))
- Render annotations by [@ymgyt](https://github.com/ymgyt) ([1f41872c](https://github.com/ymgyt/syndicationd/commit/1f41872cc07d1e86e89a05d70ae70f409c194b68))
- Stylize requirement lavel by [@ymgyt](https://github.com/ymgyt) ([324d599c](https://github.com/ymgyt/syndicationd/commit/324d599c119b1c10aa87ea7417622ae48850f7c5))
- Handle feed update by [@ymgyt](https://github.com/ymgyt) ([b0c49072](https://github.com/ymgyt/syndicationd/commit/b0c49072a02985582af37ed094d7026b43c39853))
- Normalize category by [@ymgyt](https://github.com/ymgyt) ([b25a147e](https://github.com/ymgyt/syndicationd/commit/b25a147eb02385c78e4509249cd1b6ab0caab02f))
- Show annotations in feed detail by [@ymgyt](https://github.com/ymgyt) ([cb0db4ac](https://github.com/ymgyt/syndicationd/commit/cb0db4ac5616ed93c16b511171bf3d72f4466075))
- Add entries requirement filter by [@ymgyt](https://github.com/ymgyt) ([5d49d7f4](https://github.com/ymgyt/syndicationd/commit/5d49d7f4757628cff7a8810175bb5cc2692137ae))
- Add feeds requirement filter by [@ymgyt](https://github.com/ymgyt) ([7d4b3e5c](https://github.com/ymgyt/syndicationd/commit/7d4b3e5c5ea4643d5624b0ce492ec94360799c37))
- Add category filter by [@ymgyt](https://github.com/ymgyt) ([176fc392](https://github.com/ymgyt/syndicationd/commit/176fc392a5385192d017fec4873c90ad4a92b3cf))
- Add arrow keymap by [@ymgyt](https://github.com/ymgyt) ([952a3229](https://github.com/ymgyt/syndicationd/commit/952a32294411d6596684a13aca171732f6b038d8))

### Bug Fixes

- Remove unsubscribed category from filter by [@ymgyt](https://github.com/ymgyt) ([6f5b2cb4](https://github.com/ymgyt/syndicationd/commit/6f5b2cb40dc74de3a833bba8f3ec25b52adfcf3b))

### Miscellaneous Tasks

- Add pacman to oranda install section by [@ymgyt](https://github.com/ymgyt) ([873254cd](https://github.com/ymgyt/syndicationd/commit/873254cd5fa9c8667e8043b3d1462faeb0ff0c0a))
- Fix check command typo by [@ymgyt](https://github.com/ymgyt) ([59ed1b83](https://github.com/ymgyt/syndicationd/commit/59ed1b83f911447144e648bbab2657c1d8f3bf59))
- Make table column capital consistent by [@ymgyt](https://github.com/ymgyt) ([a9b7eff9](https://github.com/ymgyt/syndicationd/commit/a9b7eff978dfc44f1bc0511bb4c465f21939ec83))
- Capitalize feed detail columns by [@ymgyt](https://github.com/ymgyt) ([ed9f0668](https://github.com/ymgyt/syndicationd/commit/ed9f0668d4d970c9181712336d9863f222aaaa4c))
- Remove feed prefix from feed detail component by [@ymgyt](https://github.com/ymgyt) ([09a0a410](https://github.com/ymgyt/syndicationd/commit/09a0a41082be9ad26340e0a560793bb697cdf21f))
- Increase fetched entries count by [@ymgyt](https://github.com/ymgyt) ([e5177160](https://github.com/ymgyt/syndicationd/commit/e5177160ace15c54a17c8bad070a1767a4fb76b8))
- Fix typo by [@ymgyt](https://github.com/ymgyt) ([a70475ec](https://github.com/ymgyt/syndicationd/commit/a70475ec3a3dc284b1a209ace81bd29dcaaee00d))
- Change category filter keymap by [@ymgyt](https://github.com/ymgyt) ([8a736ed4](https://github.com/ymgyt/syndicationd/commit/8a736ed46d970ec5e245bff1a8fa3ac7adaad21f))
- Create symlink to categories.toml by [@ymgyt](https://github.com/ymgyt) ([c990584c](https://github.com/ymgyt/syndicationd/commit/c990584c4865242a6fe97d72b63c3a2ba1b36616))

### Refactor

- Add helix like keymaps by [@ymgyt](https://github.com/ymgyt) ([257beaad](https://github.com/ymgyt/syndicationd/commit/257beaad941844b931140d8b967812cf41ce2e22))
- Resolve key events using an arrary of keymaps by [@ymgyt](https://github.com/ymgyt) ([311e3848](https://github.com/ymgyt/syndicationd/commit/311e38480f2959535cf9c7302cf335155fa15d6c))

### Testing

- Fix integration by [@ymgyt](https://github.com/ymgyt) ([0348ec21](https://github.com/ymgyt/syndicationd/commit/0348ec21b1605371c3532c648a29aca92680f7f0))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.10...synd-term-v0.2.0


## [v0.1.10] - 2024-03-19

### Features

- Make the order of keymap help consistent by [@ymgyt](https://github.com/ymgyt) ([76d385e3](https://github.com/ymgyt/syndicationd/commit/76d385e31d3f46513b4c5c39e6166f72874f16be))
- Change time format delimiter by [@ymgyt](https://github.com/ymgyt) ([b0768bc1](https://github.com/ymgyt/syndicationd/commit/b0768bc11980f9a620b6857639a33a0c01cc3fe8))
- Change entry column name by [@ymgyt](https://github.com/ymgyt) ([fd15bf63](https://github.com/ymgyt/syndicationd/commit/fd15bf633dab04159b69754901b6962cdc3f6d38))

### Miscellaneous Tasks

- Set log level for the credential restore process to debug by [@ymgyt](https://github.com/ymgyt) ([30dc7a2f](https://github.com/ymgyt/syndicationd/commit/30dc7a2f71e34ce431435acdc21c170c86c677b0))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.9...synd-term-v0.1.10


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

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.8...synd-term-v0.1.9


## [v0.1.8] - 2024-03-17

### Features

- Use nerd fond in prompt by [@ymgyt](https://github.com/ymgyt) ([b864e277](https://github.com/ymgyt/syndicationd/commit/b864e27793087d12ea63a5215df509c25854ac46))
- Use nerd font in columns by [@ymgyt](https://github.com/ymgyt) ([278fbbe8](https://github.com/ymgyt/syndicationd/commit/278fbbe833abd770d25c41f7e9e4267514ba2714))
- Support google login by [@ymgyt](https://github.com/ymgyt) ([a55c3109](https://github.com/ymgyt/syndicationd/commit/a55c31094a723e6541300898b8dab875b11a6f4a))
- Error if google jwt email is not verified by [@ymgyt](https://github.com/ymgyt) ([a8ee97d0](https://github.com/ymgyt/syndicationd/commit/a8ee97d05714f9ac9f54df61ddfc8ea1bca2cea8))
- Add nerd font to feed detail component by [@ymgyt](https://github.com/ymgyt) ([1379a297](https://github.com/ymgyt/syndicationd/commit/1379a297d6129675225f16c578aa51930293cda9))

### Refactor

- Rename device flow poll method by [@ymgyt](https://github.com/ymgyt) ([19fe8c4f](https://github.com/ymgyt/syndicationd/commit/19fe8c4fbce84cc8dab6678d38653fa304bd26ff))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.7...synd-term-v0.1.8


## [v0.1.7] - 2024-03-12

### Features

- Show first graphql error in ui by [@ymgyt](https://github.com/ymgyt) ([ca29ea02](https://github.com/ymgyt/syndicationd/commit/ca29ea021937f9818555dd64659041da04762f15))
- Add export command by [@ymgyt](https://github.com/ymgyt) ([9bb73182](https://github.com/ymgyt/syndicationd/commit/9bb731820e1f064f1d5776f5285ea57785596006))
- Print export json schema by [@ymgyt](https://github.com/ymgyt) ([3023c3f7](https://github.com/ymgyt/syndicationd/commit/3023c3f74824d4001b7d684b53b3e4e978384302))

### Miscellaneous Tasks

- Configure oranda changelog by [@ymgyt](https://github.com/ymgyt) ([1aecb8ce](https://github.com/ymgyt/syndicationd/commit/1aecb8ce5a31b766d63d8bb283e993508e379608))
- Specify features to build by [@ymgyt](https://github.com/ymgyt) ([b7db28d1](https://github.com/ymgyt/syndicationd/commit/b7db28d1eb796899a48cf23366499e287fe775fa))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.6...synd-term-v0.1.7


## [v0.1.6] - 2024-02-28

### Features

- Add move to first/last commnad by [@ymgyt](https://github.com/ymgyt) ([4bc7f482](https://github.com/ymgyt/syndicationd/commit/4bc7f482d10e52339057784052194d8ddeff30b5))

### Miscellaneous Tasks

- Read changelog by [@ymgyt](https://github.com/ymgyt) ([9095f676](https://github.com/ymgyt/syndicationd/commit/9095f6764cf8ee1bf2acff85f4df4250bb0e4167))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.5...synd-term-v0.1.6


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

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.3...synd-term-v0.1.5


## [v0.1.3] - 2024-02-19

### Features

- Improve feed url parse by [@ymgyt](https://github.com/ymgyt) ([460e87d0](https://github.com/ymgyt/syndicationd/commit/460e87d00f9acd83a922a97c339a620c0037c14f))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.2...synd-term-v0.1.3


## [v0.1.2] - 2024-02-19

### Features

- Change log directive env key by [@ymgyt](https://github.com/ymgyt) ([6da13de1](https://github.com/ymgyt/syndicationd/commit/6da13de165e3ec57e3c15c08dc6f8237debe082e))

### Miscellaneous Tasks

- Use include by [@ymgyt](https://github.com/ymgyt) ([cdff7a76](https://github.com/ymgyt/syndicationd/commit/cdff7a762d6cca85160f01b6f646d8baa6e47e59))

### Refactor

- Remove comment by [@ymgyt](https://github.com/ymgyt) ([2b042c69](https://github.com/ymgyt/syndicationd/commit/2b042c696d5c64c3dd2608e4382f50e4a76ed38c))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.1...synd-term-v0.1.2


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

https://github.com/ymgyt/syndicationd/compare/...synd-term-v0.1.1


<!-- generated by git-cliff -->
