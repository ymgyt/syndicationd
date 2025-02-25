# Changelog

All notable changes to this project will be documented in this file.

## [synd-feed-v0.3.5] - 2025-01-25

### ⚙️ Miscellaneous Tasks

- Reorganize release tasks by [@ymgyt](https://github.com/ymgyt) ([e920156a](https://github.com/ymgyt/syndicationd/commit/e920156aab15b5a839cadf5e56af2883c9c5708f))
- Refactor justfile by [@ymgyt](https://github.com/ymgyt) ([56b21266](https://github.com/ymgyt/syndicationd/commit/56b21266d79131d63e2cb6ce4136f75f8865f262))
- Appease clippy by [@ymgyt](https://github.com/ymgyt) ([133fdb01](https://github.com/ymgyt/syndicationd/commit/133fdb0155c9391b68f60a5a17daefed8db73848))
- Appease clippy by [@ymgyt](https://github.com/ymgyt) ([ec82b95e](https://github.com/ymgyt/syndicationd/commit/ec82b95e05ac4b12568422f2ea382473fba3974b))
- Release by [@](https://github.com/) ([8dfc462c](https://github.com/ymgyt/syndicationd/commit/8dfc462c8d4c8e16646debca8f2eedfcd2feb4ff))

https://github.com/ymgyt/syndicationd/compare/synd-feed-v0.3.4...synd-feed-v0.3.5


## [synd-feed-v0.3.4] - 2024-08-31

### 📡 Features

- Add import command (#132) by [@ymgyt](https://github.com/ymgyt) ([3513a253](https://github.com/ymgyt/syndicationd/commit/3513a2530b7ff3ecf8bc75ef1f03a49f34a01a81))

https://github.com/ymgyt/syndicationd/compare/synd-feed-v0.3.3...synd-feed-v0.3.4


## [synd-feed-v0.3.3] - 2024-06-18

### 📡 Features

- Use own FeedType instead of feed_rs by [@ymgyt](https://github.com/ymgyt) ([2321e4aa](https://github.com/ymgyt/syndicationd/commit/2321e4aa21c071edd4cd79b1d1b3627ac994c68d))
- Impl FeedUrl::parse by [@ymgyt](https://github.com/ymgyt) ([6db51268](https://github.com/ymgyt/syndicationd/commit/6db512681009eb8b9c19d20bc69d4bf74f383920))

### 🐛 Bug Fixes

- Typo by [@ymgyt](https://github.com/ymgyt) ([63660624](https://github.com/ymgyt/syndicationd/commit/63660624c52631531634bb7d4c470a984de1eb90))

### ⚙️ Miscellaneous Tasks

- Enable tokio macros feature by [@ymgyt](https://github.com/ymgyt) ([3eef2e71](https://github.com/ymgyt/syndicationd/commit/3eef2e71566c98693467bc7ece64fd43f325d954))

### 🔧 Testing

- Impl fake::Dummy by [@ymgyt](https://github.com/ymgyt) ([5917e31c](https://github.com/ymgyt/syndicationd/commit/5917e31c722c6d7e20d26e8f8ec4ea20337881a2))
- Add types test cases by [@ymgyt](https://github.com/ymgyt) ([9cfd8a6b](https://github.com/ymgyt/syndicationd/commit/9cfd8a6bef188a5c92fafd89316b71aeb5209ddd))
- Add feed_rs error mapping test case by [@ymgyt](https://github.com/ymgyt) ([e3f8c75f](https://github.com/ymgyt/syndicationd/commit/e3f8c75fa4198ccb51d8bbe8e2881e1ff98b5d3a))
- Add pperiodic refresher test case by [@ymgyt](https://github.com/ymgyt) ([6e9a19da](https://github.com/ymgyt/syndicationd/commit/6e9a19da92dfe5d006756d19d8c4ed1bdd9690c5))

### 🧹 Refactor

- Remove fetch_feeds_parallel from FetchFeed trait by [@ymgyt](https://github.com/ymgyt) ([a18efcea](https://github.com/ymgyt/syndicationd/commit/a18efceadb71577387696c5970ffcc07cc950fa9))

https://github.com/ymgyt/syndicationd/compare/synd-feed-v0.3.2...synd-feed-v0.3.3


## [synd-feed-v0.3.2] - 2024-05-06

### 📡 Features

- Impl Borrow<Url> for FeedUrl by [@ymgyt](https://github.com/ymgyt) ([d733b165](https://github.com/ymgyt/syndicationd/commit/d733b16533821b0bbb94b2fc51683109fd710d92))

### 🐛 Bug Fixes

- Fix cache size metrics emission by [@ymgyt](https://github.com/ymgyt) ([4396f9fc](https://github.com/ymgyt/syndicationd/commit/4396f9fc7583c55a94064675aa24049cb1e1d83d))

### ⚙️ Miscellaneous Tasks

- Remove comment outed code by [@ymgyt](https://github.com/ymgyt) ([f10f7913](https://github.com/ymgyt/syndicationd/commit/f10f79134a433f5c08d14a568945facd59fa393a))

### 🧹 Refactor

- Remove todo! macro by [@ymgyt](https://github.com/ymgyt) ([ebc2090d](https://github.com/ymgyt/syndicationd/commit/ebc2090dfcba6a9cba5758deecfd4f8b6d993df0))
- Rename parse module to service by [@ymgyt](https://github.com/ymgyt) ([256542d9](https://github.com/ymgyt/syndicationd/commit/256542d9955811eac0c26b350f528cce1106dd50))

https://github.com/ymgyt/syndicationd/compare/synd-feed-v0.3.1...synd-feed-v0.3.2


## [synd-feed-v0.3.1] - 2024-04-29

### 🐛 Bug Fixes

- Fix cache metrics name prefix by [@ymgyt](https://github.com/ymgyt) ([7e48a11e](https://github.com/ymgyt/syndicationd/commit/7e48a11e4a07ac67ba3e9cb8deac05f69abb626f))
- Emit cache metrics of the differences from the last time by [@ymgyt](https://github.com/ymgyt) ([5ea57aff](https://github.com/ymgyt/syndicationd/commit/5ea57aff46a149b69b5bfe814f13bd8c24f209b9))

https://github.com/ymgyt/syndicationd/compare/synd-feed-v0.3.0...synd-feed-v0.3.1


## [synd-feed-v0.3.0] - 2024-04-20

### 📡 Features

- Add periodic cache refresher by [@ymgyt](https://github.com/ymgyt) ([d831d3ee](https://github.com/ymgyt/syndicationd/commit/d831d3ee80dab01c004ba37d7e30c64f9750e6de))

### 🧹 Refactor

- Use FeedUrl instead of String by [@ymgyt](https://github.com/ymgyt) ([759950b9](https://github.com/ymgyt/syndicationd/commit/759950b9ff64d0b407483c55ebf733eceb6b6d09))
- Make tests module consistent by [@ymgyt](https://github.com/ymgyt) ([5b85455e](https://github.com/ymgyt/syndicationd/commit/5b85455e15b29bafd1c357ec0ecba8b1e3eed0cc))

https://github.com/ymgyt/syndicationd/compare/synd-feed-v0.2.0...synd-feed-v0.3.0


## [synd-feed-v0.2.0] - 2024-04-17

### 📡 Features

- Add feed annotations by [@ymgyt](https://github.com/ymgyt) ([6f9f1fe0](https://github.com/ymgyt/syndicationd/commit/6f9f1fe0919912138f658ff22deedba7e0c7126a))
- Add Annotated::project by [@ymgyt](https://github.com/ymgyt) ([ddb1c0aa](https://github.com/ymgyt/syndicationd/commit/ddb1c0aac537aa56fafb3432a346fc83ab33cd4a))
- Impl Display for annotation types by [@ymgyt](https://github.com/ymgyt) ([d68aa81d](https://github.com/ymgyt/syndicationd/commit/d68aa81de61cdd72731e8f68b6b1d440efc67ec9))
- Add requirement up/down by [@ymgyt](https://github.com/ymgyt) ([10e455d2](https://github.com/ymgyt/syndicationd/commit/10e455d251b1598501d5527423ba74c4b18920d8))

### ⚙️ Miscellaneous Tasks

- Disable cargo-dist by [@ymgyt](https://github.com/ymgyt) ([7aeeea59](https://github.com/ymgyt/syndicationd/commit/7aeeea591040165444dbb59868760e02f6628b6f))

https://github.com/ymgyt/syndicationd/compare/synd-feed-v0.1.5...synd-feed-v0.2.0


## [synd-feed-v0.1.5] - 2024-02-25

### ⚙️ Miscellaneous Tasks

- Trim prefix from changelog by [@ymgyt](https://github.com/ymgyt) ([95d44877](https://github.com/ymgyt/syndicationd/commit/95d448773ec7ab009fbece0928854364679b6f2c))
- Add homepage to package metadata by [@ymgyt](https://github.com/ymgyt) ([4bfdb49e](https://github.com/ymgyt/syndicationd/commit/4bfdb49e317e18ff6345ce1b8e8071f0497a1a5f))

https://github.com/ymgyt/syndicationd/compare/synd-feed-v0.1.4...synd-feed-v0.1.5


## [synd-feed-v0.1.4] - 2024-02-23

### 📡 Features

- Support json feed website url parse by [@ymgyt](https://github.com/ymgyt) ([7e8db96c](https://github.com/ymgyt/syndicationd/commit/7e8db96c05d33604381168e85f929063b5a85bdd))
- Handle subscribe feed error by [@ymgyt](https://github.com/ymgyt) ([d6abb26e](https://github.com/ymgyt/syndicationd/commit/d6abb26eb7ea75ba479f07cb83ff680a1708c6af))
- Use entry updated if published is none by [@ymgyt](https://github.com/ymgyt) ([2b16b51c](https://github.com/ymgyt/syndicationd/commit/2b16b51c3cadb7b0dd74a848ae43ff078372b678))
- Add generators to feed by [@ymgyt](https://github.com/ymgyt) ([3f0f8b43](https://github.com/ymgyt/syndicationd/commit/3f0f8b4303e2698a9ae364c2c114f0f6d83ffc33))
- Return entry content by [@ymgyt](https://github.com/ymgyt) ([9f462854](https://github.com/ymgyt/syndicationd/commit/9f462854a1e0d46af515a91237fb3660c72c1fad))

https://github.com/ymgyt/syndicationd/compare/synd-feed-v0.1.3...synd-feed-v0.1.4


## [synd-feed-v0.1.3] - 2024-02-20

### 📡 Features

- Instrument fetch_feed span by [@ymgyt](https://github.com/ymgyt) ([b5cdacb7](https://github.com/ymgyt/syndicationd/commit/b5cdacb7d5a21012b1273a34af419abf6143251d))

https://github.com/ymgyt/syndicationd/compare/synd-feed-v0.1.2...synd-feed-v0.1.3


## [synd-feed-v0.1.2] - 2024-02-19

### 📚 Documentation

- Fix typo by [@ymgyt](https://github.com/ymgyt) ([d611d33a](https://github.com/ymgyt/syndicationd/commit/d611d33af376e593d24533378845c565dadd4e5e))

https://github.com/ymgyt/syndicationd/compare/synd-feed-v0.1.1...synd-feed-v0.1.2


## [synd-feed-v0.1.1] - 2024-02-12

### 📡 Features

- Remove unsubscribed entries by [@ymgyt](https://github.com/ymgyt) ([d29ba92e](https://github.com/ymgyt/syndicationd/commit/d29ba92e929d9d1348fa114ac2bdf210b76c5a1b))

### ⚙️ Miscellaneous Tasks

- Format toml by [@ymgyt](https://github.com/ymgyt) ([36677745](https://github.com/ymgyt/syndicationd/commit/3667774506106fe0f38d77efac9f4b27c70090aa))
- Configure release flow by [@ymgyt](https://github.com/ymgyt) ([855d1063](https://github.com/ymgyt/syndicationd/commit/855d1063f5b476433fe0a7ab352b72d63a749e2e))
- Use hyphen as package name instead of underscore by [@ymgyt](https://github.com/ymgyt) ([0a8ed059](https://github.com/ymgyt/syndicationd/commit/0a8ed05997790f9f05c932c92fa2b2b2d74065a9))
- Update CHANGELOG by [@ymgyt](https://github.com/ymgyt) ([ea469bfe](https://github.com/ymgyt/syndicationd/commit/ea469bfefec9adf294667f4131538d496a6b017d))

### 🧹 Refactor

- Rename crates by [@ymgyt](https://github.com/ymgyt) ([ce0982e4](https://github.com/ymgyt/syndicationd/commit/ce0982e497647b23dcf07e39d525121bcd9ac1fa))
- Use clippy pedantic by [@ymgyt](https://github.com/ymgyt) ([328ddade](https://github.com/ymgyt/syndicationd/commit/328ddadebbad5381271c5e84cce2d6888252e70c))

https://github.com/ymgyt/syndicationd/compare/...synd-feed-v0.1.1


<!-- generated by git-cliff -->
