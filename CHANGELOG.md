# Changelog

All notable changes to this project will be documented in this file.

## [v0.4.0-rc.1] - 2026-07-29

### 📡 Features

- Migrate to sqlite by [@ymgyt](https://github.com/ymgyt) ([9c33ee88](https://github.com/ymgyt/syndicationd/commit/9c33ee8819a3e45ac3452b115c7a14a0c4d57228))
- Add local serving mode by [@ymgyt](https://github.com/ymgyt) ([1fc97099](https://github.com/ymgyt/syndicationd/commit/1fc97099fd9707ba0c75ee1362c7fa5733cebb4c))
- Launch local api backend by [@ymgyt](https://github.com/ymgyt) ([5e8b5d91](https://github.com/ymgyt/syndicationd/commit/5e8b5d9147ef6a945b65e425cc5a0da833413dd4))
- Default to local backend by [@ymgyt](https://github.com/ymgyt) ([6d866eba](https://github.com/ymgyt/syndicationd/commit/6d866eba574b434a72baf8687eed2780a8aa41be))
- Support local backend in port commands by [@ymgyt](https://github.com/ymgyt) ([7c4a0ec6](https://github.com/ymgyt/syndicationd/commit/7c4a0ec6729f213ae3c7c86f3fb9b3829d93a901))
- Add feed registry domain by [@ymgyt](https://github.com/ymgyt) ([82b90c56](https://github.com/ymgyt/syndicationd/commit/82b90c56c59a92838928cd7385f1f82d54b88488))
- Add sqlite feed registry store by [@ymgyt](https://github.com/ymgyt) ([5ea678ad](https://github.com/ymgyt/syndicationd/commit/5ea678ad476bff58492b930b29074fc40a70d78d))
- Expose feed registry events by [@ymgyt](https://github.com/ymgyt) ([ea67403e](https://github.com/ymgyt/syndicationd/commit/ea67403edd1dd64c826dc727b2956e48fe46fbdd))
- Consume feed registry events by [@ymgyt](https://github.com/ymgyt) ([51de5c02](https://github.com/ymgyt/syndicationd/commit/51de5c022e1b0a676e297268ea4e6aeaedb05228))
- Rename sqlite database option by [@ymgyt](https://github.com/ymgyt) ([a7026cdb](https://github.com/ymgyt/syndicationd/commit/a7026cdbd0d86290f4c7c7c8b63016c02013d433))
- Add deterministic synd entry id by [@ymgyt](https://github.com/ymgyt) ([04dff13e](https://github.com/ymgyt/syndicationd/commit/04dff13e05c6257248970b8f48faedb56204075b))
- Add event runtime for commands by [@ymgyt](https://github.com/ymgyt) ([9932ffc2](https://github.com/ymgyt/syndicationd/commit/9932ffc221854b3f5c45574dfadead9bb9fa3be6))
- Add sqlite event journal by [@ymgyt](https://github.com/ymgyt) ([b7a72d61](https://github.com/ymgyt/syndicationd/commit/b7a72d6105cd2731ec55b24be8bf804a11a609f5))
- Launch and control singleton daemon by [@ymgyt](https://github.com/ymgyt) ([12d5e1d9](https://github.com/ymgyt/syndicationd/commit/12d5e1d9b29508b7d06c7c68377f39f51aad6207))
- Add configurable keymap v2 by [@ymgyt](https://github.com/ymgyt) ([bfacc9fe](https://github.com/ymgyt/syndicationd/commit/bfacc9feacaf3f225e919f3635a82db98d8afc72))
- Project crawl targets from subscriptions by [@ymgyt](https://github.com/ymgyt) ([349068d0](https://github.com/ymgyt/syndicationd/commit/349068d005d494774dcbf0edb7a9f566edebd531))
- Wake crawl scheduler from target events by [@ymgyt](https://github.com/ymgyt) ([b31aadc1](https://github.com/ymgyt/syndicationd/commit/b31aadc1d765e62911cca8bb7f5f5bf91babc735))
- Renew daemon session leases by [@ymgyt](https://github.com/ymgyt) ([931d9f4e](https://github.com/ymgyt/syndicationd/commit/931d9f4eacf50adce3f9670fec1c6111dde3aeb1))
- Enqueue crawl jobs from scheduler by [@ymgyt](https://github.com/ymgyt) ([aeffb114](https://github.com/ymgyt/syndicationd/commit/aeffb114829fd251146fa67e804e1055c85ae883))
- Configure daemon session lifecycle by [@ymgyt](https://github.com/ymgyt) ([12d8fe7e](https://github.com/ymgyt/syndicationd/commit/12d8fe7e634d97cfb506c77310ee77e078360930))
- Configure runtime root by [@ymgyt](https://github.com/ymgyt) ([e44dfc45](https://github.com/ymgyt/syndicationd/commit/e44dfc456bd03facf209431c29b6651419a4d9fc))
- Add daemon status json output by [@ymgyt](https://github.com/ymgyt) ([6d0215a5](https://github.com/ymgyt/syndicationd/commit/6d0215a5034d563f5c0eb4c12eedd7c01727047e))
- Include daemon launch diagnostics by [@ymgyt](https://github.com/ymgyt) ([8d688e48](https://github.com/ymgyt/syndicationd/commit/8d688e48bde753f7dc1b12cfba3ab65a19bffa25))
- Report daemon session status by [@ymgyt](https://github.com/ymgyt) ([11b7f0d3](https://github.com/ymgyt/syndicationd/commit/11b7f0d3aff12180c9107eb02b93f172703309a8))
- Diagnose runtime daemon placement by [@ymgyt](https://github.com/ymgyt) ([1d3aab53](https://github.com/ymgyt/syndicationd/commit/1d3aab533e69877a9db1955b4ff1f35be6aaf211))
- Start crawl worker pool by [@ymgyt](https://github.com/ymgyt) ([e3970d33](https://github.com/ymgyt/syndicationd/commit/e3970d33b0db383f1bca22ba7238e814940f83c6))
- Add forced daemon shutdown by [@ymgyt](https://github.com/ymgyt) ([af2f2e60](https://github.com/ymgyt/syndicationd/commit/af2f2e606a79dae054f1b1c46d181ac19f8cba1f))
- Claim crawl jobs from worker pool by [@ymgyt](https://github.com/ymgyt) ([085a58a4](https://github.com/ymgyt/syndicationd/commit/085a58a444fda9a50d0e1cf00fd0efa5bd9a46b4))
- Persist crawl job completions by [@ymgyt](https://github.com/ymgyt) ([44c198f5](https://github.com/ymgyt/syndicationd/commit/44c198f51beb03ed35a3543335b188d441d8443b))
- Project feed state from crawl results by [@ymgyt](https://github.com/ymgyt) ([92bf6fc1](https://github.com/ymgyt/syndicationd/commit/92bf6fc1442a54264f36ed9c1da1efa91b0497ae))
- Project timeline by [@ymgyt](https://github.com/ymgyt) ([8e48ebc8](https://github.com/ymgyt/syndicationd/commit/8e48ebc8a5320ae3e8e33c66430896d20b072fa2))
- Project timeline membership events by [@ymgyt](https://github.com/ymgyt) ([2d454276](https://github.com/ymgyt/syndicationd/commit/2d454276ab9c81497ae130f465492717edf0c79d))
- Unify feed event subscription channel by [@ymgyt](https://github.com/ymgyt) ([646249d3](https://github.com/ymgyt/syndicationd/commit/646249d334733f7ef305db6b4234677f41103251))
- Classify processor failures by [@ymgyt](https://github.com/ymgyt) ([ecbc6365](https://github.com/ymgyt/syndicationd/commit/ecbc636543d1351af1dcf1721fa5ad5dc72ce072))
- Stream subscribe progress from registry events by [@ymgyt](https://github.com/ymgyt) ([16d14f63](https://github.com/ymgyt/syndicationd/commit/16d14f63592c2589b2c5967e46cc21f1038980d0))
- Add lifecycle logging by [@ymgyt](https://github.com/ymgyt) ([3d29abda](https://github.com/ymgyt/syndicationd/commit/3d29abda95cc1c81a893997c03ab752b3c2df198))
- Return unsubscribe disposition by [@ymgyt](https://github.com/ymgyt) ([a9d8a3d0](https://github.com/ymgyt/syndicationd/commit/a9d8a3d051ff80629255e1c74e8ab6ca1c940d59))
- Expose entry ids by [@ymgyt](https://github.com/ymgyt) ([2e67f44b](https://github.com/ymgyt/syndicationd/commit/2e67f44bf21fbc8edd930cf96b8b0e828ea100f2))
- Sync timeline incrementally by seq changes by [@ymgyt](https://github.com/ymgyt) ([d0e7be8e](https://github.com/ymgyt/syndicationd/commit/d0e7be8eb5ceb047830c0d54730ef1f7ef0a3ce8))
- Sync timeline incrementally by seq changes by [@ymgyt](https://github.com/ymgyt) ([c662d634](https://github.com/ymgyt/syndicationd/commit/c662d634646d46a7b4198cdf62f718403eb74598))
- Redraw screen with R key by [@ymgyt](https://github.com/ymgyt) ([612d9cff](https://github.com/ymgyt/syndicationd/commit/612d9cffa40e4ff7013644fe652c324ecf591599))
- Expose feed construction by [@ymgyt](https://github.com/ymgyt) ([bf411702](https://github.com/ymgyt/syndicationd/commit/bf4117026c020e61ea06a107589c412ee7e4ecb2))
- Project complete feed state by [@ymgyt](https://github.com/ymgyt) ([d6f0f6aa](https://github.com/ymgyt/syndicationd/commit/d6f0f6aa512eeb3741801e3c4359d8f46afaa31c))
- Add diagnostic lifecycle logging by [@ymgyt](https://github.com/ymgyt) ([5bb4a35b](https://github.com/ymgyt/syndicationd/commit/5bb4a35bcddb78ecbd433d8a28daa872899875ee))

### 🐛 Bug Fixes

- Handle gh not found gql error (#310) by [@ymgyt](https://github.com/ymgyt) ([acd00677](https://github.com/ymgyt/syndicationd/commit/acd00677f754fe43a0dd3962f3c32260c7056520))
- Satisfy clippy warnings by [@ymgyt](https://github.com/ymgyt) ([31276309](https://github.com/ymgyt/syndicationd/commit/31276309e6003ac7cbdd8c988bdbd209bc2b18bd))
- Avoid latest tag for prerelease images by [@ymgyt](https://github.com/ymgyt) ([268c3252](https://github.com/ymgyt/syndicationd/commit/268c3252142542c6fdcafde42620a3015409618e))
- Recover incompatible daemon by [@ymgyt](https://github.com/ymgyt) ([f07676ea](https://github.com/ymgyt/syndicationd/commit/f07676ea0966b0d4ea6345fe678598ac8db72180))
- Classify daemon session failures by [@ymgyt](https://github.com/ymgyt) ([aa3b8a84](https://github.com/ymgyt/syndicationd/commit/aa3b8a84f35ab0670b012df70d49153ea78ecece))
- Classify missing daemon endpoint root by [@ymgyt](https://github.com/ymgyt) ([4128446b](https://github.com/ymgyt/syndicationd/commit/4128446b65f7baae928d5ffd5baf71923ab96094))
- Report command failures plainly by [@ymgyt](https://github.com/ymgyt) ([63e151a3](https://github.com/ymgyt/syndicationd/commit/63e151a33beca0fdc787c9b8b775b462c9b61a49))
- Tighten daemon control surface by [@ymgyt](https://github.com/ymgyt) ([b7184658](https://github.com/ymgyt/syndicationd/commit/b7184658350b8060ec5f03735efe3756a6265f99))
- Box daemon session handle by [@ymgyt](https://github.com/ymgyt) ([b38f2bea](https://github.com/ymgyt/syndicationd/commit/b38f2bea7b26a974d2fd512759a84c21d3ddbbcf))
- Disable ansi colors for log files by [@ymgyt](https://github.com/ymgyt) ([d279f08c](https://github.com/ymgyt/syndicationd/commit/d279f08c8d04438a513327059ff7ca2ac9384d43))
- Package the synd crate binary by [@ymgyt](https://github.com/ymgyt) ([85a21fb4](https://github.com/ymgyt/syndicationd/commit/85a21fb4f86a5b546472368b150bf19eab0558af))
- Give every timeline change a unique seq by [@ymgyt](https://github.com/ymgyt) ([95863902](https://github.com/ymgyt/syndicationd/commit/95863902e59e4ba38fc51d09f2789217dfcd3316))
- Normalize shifted chars in keystroke identity by [@ymgyt](https://github.com/ymgyt) ([70f131c0](https://github.com/ymgyt/syndicationd/commit/70f131c06b69406ef2fc42a092c5d40590554f39))
- Drop cursor position query from force redraw by [@ymgyt](https://github.com/ymgyt) ([c1bbb0a8](https://github.com/ymgyt/syndicationd/commit/c1bbb0a87766301cf3061f1d32ccd52ef10cdc4f))
- Correct default requirement constant typo by [@ymgyt](https://github.com/ymgyt) ([4356e0d3](https://github.com/ymgyt/syndicationd/commit/4356e0d3abab0717c868dcd37e81e4f82645c4e3))
- Align distribution names with synd by [@ymgyt](https://github.com/ymgyt) ([89356bba](https://github.com/ymgyt/syndicationd/commit/89356bbaa48e7244d14424e7330eecf73ecb6fd5))

### Infra

- Remove Terraform hosting setup by [@ymgyt](https://github.com/ymgyt) ([26fd6a82](https://github.com/ymgyt/syndicationd/commit/26fd6a8269c4853671322b38907d63d888867351))

### ⚙️ Miscellaneous Tasks

- Trim package name from version in changelog by [@ymgyt](https://github.com/ymgyt) ([46b17de1](https://github.com/ymgyt/syndicationd/commit/46b17de1914167c3f78add42d21f93b6ab496f07))
- Bump nom from 8.0.0-alpha2 to 8.0.0 by [@ymgyt](https://github.com/ymgyt) ([e675e9ca](https://github.com/ymgyt/syndicationd/commit/e675e9ca6ee5c3058d91986e60bd4e1928e4d349))
- Bump rust from 1.84.0 to 1.85.0 by [@ymgyt](https://github.com/ymgyt) ([a2d49805](https://github.com/ymgyt/syndicationd/commit/a2d4980591f8f8da87ed825543b6dbae34dc7919))
- Bump edition from 2021 to 2024 by [@ymgyt](https://github.com/ymgyt) ([2162ff03](https://github.com/ymgyt/syndicationd/commit/2162ff038b48c900ce3ca7343433dd7d3cce3ddf))
- Add axum group to dependabot by [@ymgyt](https://github.com/ymgyt) ([e2acae29](https://github.com/ymgyt/syndicationd/commit/e2acae298e76326c0c5ad2772273f482bb0f7cc4))
- Bump crane from 0.20.0 to 0.20.1 by [@ymgyt](https://github.com/ymgyt) ([73439a28](https://github.com/ymgyt/syndicationd/commit/73439a28c87551d0da666eacd22920e81c673658))
- Fix axum group patterns by [@ymgyt](https://github.com/ymgyt) ([0031b658](https://github.com/ymgyt/syndicationd/commit/0031b658fe4af8e6d567403f534f8100721af299))
- Bump rust from 1.85.0 to 1.86.0 by [@ymgyt](https://github.com/ymgyt) ([e2014ded](https://github.com/ymgyt/syndicationd/commit/e2014ded49bc02819aba8b3223970a07dcb2c530))
- Use flake update to update lockfile by [@ymgyt](https://github.com/ymgyt) ([c3b366a0](https://github.com/ymgyt/syndicationd/commit/c3b366a0b93a94d192a7bffffe027fcb3b4553ae))
- Handle audit advisories by [@ymgyt](https://github.com/ymgyt) ([d29532c6](https://github.com/ymgyt/syndicationd/commit/d29532c630881f6f05809de475f7b1cfc0ab4f57))
- Bump tokio from 1.43 to 1.44.1 by [@ymgyt](https://github.com/ymgyt) ([9667ac0d](https://github.com/ymgyt/syndicationd/commit/9667ac0d6e43849eb9e0b31927f88e65f18fbcae))
- Bump opentelemetry from 0.27.0 to 0.29.0 by [@ymgyt](https://github.com/ymgyt) ([1896b24a](https://github.com/ymgyt/syndicationd/commit/1896b24a1d6fe6562427a656362f3d8e10573a78))
- Update string handling in nu script by [@ymgyt](https://github.com/ymgyt) ([86151aec](https://github.com/ymgyt/syndicationd/commit/86151aec1c0b80871837d638ea8ea9abf5142f6a))
- Set abort in release profile by [@ymgyt](https://github.com/ymgyt) ([d3930f17](https://github.com/ymgyt/syndicationd/commit/d3930f17997e346b014331489e90e6f4d559aad0))
- Update ubuntu runner in release wf by [@ymgyt](https://github.com/ymgyt) ([5ffb8940](https://github.com/ymgyt/syndicationd/commit/5ffb894025dd7c7ed4755398608e98347d5c5c3a))
- Remove kvsd crates by [@ymgyt](https://github.com/ymgyt) ([780aa05a](https://github.com/ymgyt/syndicationd/commit/780aa05aaccbc8ab31ce022924c038d7750de881))
- Replace deprecated into_path() with keep() for TempDir by [@ymgyt](https://github.com/ymgyt) ([8ae91e44](https://github.com/ymgyt/syndicationd/commit/8ae91e4432e2831dddb18beaa469bcbec65251b9))
- Bump rust from 1.86 to 1.90 by [@ymgyt](https://github.com/ymgyt) ([c814af95](https://github.com/ymgyt/syndicationd/commit/c814af9527effb6718035ab76ec44215d0cb8a04))
- Bump nixpkgs from 24.11 to 25.05 by [@ymgyt](https://github.com/ymgyt) ([553d8c97](https://github.com/ymgyt/syndicationd/commit/553d8c9707b5a35c76ebc6cdf738630552c92f5e))
- Bump nixpkgs from 25.05 to 25.11 by [@ymgyt](https://github.com/ymgyt) ([5223fb9a](https://github.com/ymgyt/syndicationd/commit/5223fb9aefaa56f6e05720a8698b684486cc95b2))
- Bump rust from 1.90.0 to 1.92.0 by [@ymgyt](https://github.com/ymgyt) ([c2cc53b7](https://github.com/ymgyt/syndicationd/commit/c2cc53b7a8f80816728cf0760f553f0a9600cbd1))
- Bump ratatui from 0.29.0 to 0.30.0 by [@ymgyt](https://github.com/ymgyt) ([b3d040ce](https://github.com/ymgyt/syndicationd/commit/b3d040ce3fa6cfc022bd14cd0bcdd7c678a1672e))
- Bump axum-server by [@ymgyt](https://github.com/ymgyt) ([9958d01f](https://github.com/ymgyt/syndicationd/commit/9958d01fec80ea68621f3f7e6fa0be43fb258d76))
- Use nixpkg-unstable to use latest cargo-audit for parsing CVSS 4 format by [@ymgyt](https://github.com/ymgyt) ([86e67cc2](https://github.com/ymgyt/syndicationd/commit/86e67cc2d5f60a68a4fa94ec5c7654f23b5d6724))
- Use nixfmt as nix formatter by [@ymgyt](https://github.com/ymgyt) ([24f1e41f](https://github.com/ymgyt/syndicationd/commit/24f1e41f0f82f61ff784a69fca3a90fe8468e6f8))
- Fmt flake by [@ymgyt](https://github.com/ymgyt) ([96dacae1](https://github.com/ymgyt/syndicationd/commit/96dacae1a0281bcae06ec180a105fe42a6aa3e5b))
- Update nixpkgs by [@ymgyt](https://github.com/ymgyt) ([b7b7da83](https://github.com/ymgyt/syndicationd/commit/b7b7da83c8837a4296f2e25cf2980eef35536e0c))
- Fmt cliff by [@ymgyt](https://github.com/ymgyt) ([7fc265c3](https://github.com/ymgyt/syndicationd/commit/7fc265c3c74b6c65be17477d28b49ecf8d6b905b))
- Fmt toml by [@ymgyt](https://github.com/ymgyt) ([9dc26b11](https://github.com/ymgyt/syndicationd/commit/9dc26b11ce06e1c2927b1d52f673095d2def2b84))
- Fix taplo wildcard arg by [@ymgyt](https://github.com/ymgyt) ([e86edbc0](https://github.com/ymgyt/syndicationd/commit/e86edbc0a439e36671471b9a4c15adac757a3845))
- Pass ci by [@ymgyt](https://github.com/ymgyt) ([844601f9](https://github.com/ymgyt/syndicationd/commit/844601f9197b4989054d697b55efd32a15f54acf))
- Update bytes to 1.11.1 to address RUSTSEC-2026-0007 by [@ymgyt](https://github.com/ymgyt) ([94d88fa0](https://github.com/ymgyt/syndicationd/commit/94d88fa0a58d1dc8eb2b0df8d30cd59706e0dfd8))
- Update time to 0.3.47 to address RUSTSEC-2026-0009 by [@ymgyt](https://github.com/ymgyt) ([9ec95f73](https://github.com/ymgyt/syndicationd/commit/9ec95f73bfe2b8059d4f7d32f8ad71047a3aff53))
- Appease audit by [@ymgyt](https://github.com/ymgyt) ([dd98f15d](https://github.com/ymgyt/syndicationd/commit/dd98f15d3324fb5f085619f6e7dc0c80ddcc8c5b))
- Update octocrab gql response handling by [@ymgyt](https://github.com/ymgyt) ([32b7bed9](https://github.com/ymgyt/syndicationd/commit/32b7bed94e4601e42e5a0dbeac76f0ff3e45cc11))
- Ignore .local dir by [@ymgyt](https://github.com/ymgyt) ([01c6aa68](https://github.com/ymgyt/syndicationd/commit/01c6aa68f27b792d33e38b41500e3148e61941f4))
- Remove unused cargo alias by [@ymgyt](https://github.com/ymgyt) ([2085bd56](https://github.com/ymgyt/syndicationd/commit/2085bd565492e1c60788024c9d1fadb6e69a0cb7))
- Remove stale just tasks by [@ymgyt](https://github.com/ymgyt) ([712c96a0](https://github.com/ymgyt/syndicationd/commit/712c96a07d218194d8032a375b330810f7a89cbc))
- Stop ignoring RUSTSEC-2024-0370 by [@ymgyt](https://github.com/ymgyt) ([dfcde671](https://github.com/ymgyt/syndicationd/commit/dfcde671d30790e24071529151e38202b8292af9))
- Update workspace lockfile by [@ymgyt](https://github.com/ymgyt) ([c35794b6](https://github.com/ymgyt/syndicationd/commit/c35794b6d7b917914f3dbc3bd64d240a05fdd795))
- Refresh flake inputs by [@ymgyt](https://github.com/ymgyt) ([303029c3](https://github.com/ymgyt/syndicationd/commit/303029c39889fe7e71168d5864e78efb6aed790f))
- Update dependabot fetch metadata by [@ymgyt](https://github.com/ymgyt) ([aa1f56c0](https://github.com/ymgyt/syndicationd/commit/aa1f56c00609bcdc8d7d38d7e74cef45100186dc))
- Update cargo-dist release workflow by [@ymgyt](https://github.com/ymgyt) ([ec178fe7](https://github.com/ymgyt/syndicationd/commit/ec178fe7f83cd5b62300f537f33af3f2ddfeac23))
- Refresh flake inputs by [@ymgyt](https://github.com/ymgyt) ([b30a465a](https://github.com/ymgyt/syndicationd/commit/b30a465a1a15d15b929ceb534e5a8ad163a2194c))
- Migrate from dependabot by [@ymgyt](https://github.com/ymgyt) ([9cad35df](https://github.com/ymgyt/syndicationd/commit/9cad35df2018580f881bc04aa1e84942291a3252))
- Update ratatui to 0.30.1 by [@ymgyt](https://github.com/ymgyt) ([4282499a](https://github.com/ymgyt/syndicationd/commit/4282499a4297e6b413da229d9119ecc1c833a098))
- Bump rust from 1.95.0 to 1.96.0 by [@ymgyt](https://github.com/ymgyt) ([d47a352f](https://github.com/ymgyt/syndicationd/commit/d47a352f9678986bab451f902fd1719bc0ff5348))
- Widen update schedule by [@ymgyt](https://github.com/ymgyt) ([d0d9307a](https://github.com/ymgyt/syndicationd/commit/d0d9307a899246bf51813d262785ca12198ddac1))
- Align rust checks by [@ymgyt](https://github.com/ymgyt) ([23ba0118](https://github.com/ymgyt/syndicationd/commit/23ba0118cf7b58aad594efaf92b23cb36598206b))
- Remove stale task by [@ymgyt](https://github.com/ymgyt) ([5bcd90c4](https://github.com/ymgyt/syndicationd/commit/5bcd90c49aa3f127c7db34d5849b966be37690ae))
- Fmt by [@ymgyt](https://github.com/ymgyt) ([a19d3002](https://github.com/ymgyt/syndicationd/commit/a19d30021b62533d4c9c970779ca5985b8fc58a4))
- Update rust to 1.97.1 by [@ymgyt](https://github.com/ymgyt) ([9d247ba8](https://github.com/ymgyt/syndicationd/commit/9d247ba85a9ab1ee8356ca785c9dd57eeb31da5f))
- Unify workspace versions by [@ymgyt](https://github.com/ymgyt) ([b99d1b59](https://github.com/ymgyt/syndicationd/commit/b99d1b5989cc7e18c0b4b2487e38d645a048c66b))
- Unify changelog generation by [@ymgyt](https://github.com/ymgyt) ([8b19d0e9](https://github.com/ymgyt/syndicationd/commit/8b19d0e9420a6ee5c8e1316b3eb7bbd3e1b1ce31))
- Prepare workspace publication by [@ymgyt](https://github.com/ymgyt) ([0928afe5](https://github.com/ymgyt/syndicationd/commit/0928afe5722dc82cb2a458cd197e76c1457d18fc))
- Update binary distribution by [@ymgyt](https://github.com/ymgyt) ([8747c2e1](https://github.com/ymgyt/syndicationd/commit/8747c2e117c6fe007d8c3e22cfcdb8800b7c37d0))
- Reorder recipe by [@ymgyt](https://github.com/ymgyt) ([049feed5](https://github.com/ymgyt/syndicationd/commit/049feed5d26e4522af1ccb0ec405add13183d24f))
- Remove x86_64-apple-darwin from dist target by [@ymgyt](https://github.com/ymgyt) ([a9285b67](https://github.com/ymgyt/syndicationd/commit/a9285b6797ae9a53619d647ae1d4845df9258423))
- Use profile add command by [@ymgyt](https://github.com/ymgyt) ([fcb710e0](https://github.com/ymgyt/syndicationd/commit/fcb710e0889d1ebbd12d3e0e5f7ffe5dcd68cab6))
- Update yanked spin dependencies by ymgyt ([0f6319ab](https://github.com/ymgyt/syndicationd/commit/0f6319ab086957dd54e58cf47c9b77e389532eee))

### 📚 Documentation

- Fix typo by [@ymgyt](https://github.com/ymgyt) ([7b1c2c5f](https://github.com/ymgyt/syndicationd/commit/7b1c2c5f972bf84d6b41316623576b9b3bed2825))
- Update README for local-first mode by [@ymgyt](https://github.com/ymgyt) ([6a3faf3f](https://github.com/ymgyt/syndicationd/commit/6a3faf3f35a51e13e21fb337877be0ac90303d35))
- Update contributor guide by [@ymgyt](https://github.com/ymgyt) ([b7176f53](https://github.com/ymgyt/syndicationd/commit/b7176f53f28774a5bd07ce45cb8f1280db02ebff))
- Refresh user-facing guide by [@ymgyt](https://github.com/ymgyt) ([f8a9c74f](https://github.com/ymgyt/syndicationd/commit/f8a9c74f4bec4ef595248b1432f965e736294723))
- Update package topology and CLI guide by [@ymgyt](https://github.com/ymgyt) ([11d96e51](https://github.com/ymgyt/syndicationd/commit/11d96e510fc79eea1f3409cb3e219ac370308593))
- Update user documentation by [@ymgyt](https://github.com/ymgyt) ([4b2e4a49](https://github.com/ymgyt/syndicationd/commit/4b2e4a49c9a392d223ec86adcda57c4aae903a98))
- Update README by [@ymgyt](https://github.com/ymgyt) ([1aa9edf2](https://github.com/ymgyt/syndicationd/commit/1aa9edf2ecd59876c64beab26db498c224f9e774))
- Update app topology by [@ymgyt](https://github.com/ymgyt) ([c0c96857](https://github.com/ymgyt/syndicationd/commit/c0c96857714ff4bcea9a852346006ed0c54cbf72))
- Prepare synd README for v0.4.0-rc.1 by [@ymgyt](https://github.com/ymgyt) ([d08c2698](https://github.com/ymgyt/syndicationd/commit/d08c2698228f8ff59a88d6dce0966491e501c930))
- Streamline README by ymgyt ([64e028e9](https://github.com/ymgyt/syndicationd/commit/64e028e91a25a4dcf1e93ab95d3eaa73c7813259))

### 🔧 Testing

- Fix select arm order in refresher test by [@ymgyt](https://github.com/ymgyt) ([28f747f0](https://github.com/ymgyt/syndicationd/commit/28f747f098ddfbcf614e73cadab77cf9dac80d3a))
- Update snapshot by [@ymgyt](https://github.com/ymgyt) ([956bec58](https://github.com/ymgyt/syndicationd/commit/956bec58fcb6dbd9e8740d925649772b1670ead9))
- Update otel snapshots by [@ymgyt](https://github.com/ymgyt) ([4337e8da](https://github.com/ymgyt/syndicationd/commit/4337e8da13df637cacbe871548d26fb09fdd1377))
- Stabilize otel metrics export test by [@ymgyt](https://github.com/ymgyt) ([762a4b6d](https://github.com/ymgyt/syndicationd/commit/762a4b6d3fc99143e1bb229f330e06a7087a40d0))
- Remove stale sqlite fixture by [@ymgyt](https://github.com/ymgyt) ([be0a8df0](https://github.com/ymgyt/syndicationd/commit/be0a8df06e1efd6adaaf33c78b5f2f6c932d4419))
- Skip cli integration test by [@ymgyt](https://github.com/ymgyt) ([64768d99](https://github.com/ymgyt/syndicationd/commit/64768d99c305aaea7875da603b441977822b51cf))
- Organize daemon session test contexts by [@ymgyt](https://github.com/ymgyt) ([c31293fe](https://github.com/ymgyt/syndicationd/commit/c31293fec980ea4bbd61c47ca907804a5866469b))
- Cover shared daemon sessions by [@ymgyt](https://github.com/ymgyt) ([eb25b703](https://github.com/ymgyt/syndicationd/commit/eb25b7039c555004b5117376f884722a12f90aed))
- Use assert_matches for pattern assertions by [@ymgyt](https://github.com/ymgyt) ([9d2d1d73](https://github.com/ymgyt/syndicationd/commit/9d2d1d73e1c1fae5359bddb1e43a471398457e7c))
- Decompose integration coverage by [@ymgyt](https://github.com/ymgyt) ([4cccea25](https://github.com/ymgyt/syndicationd/commit/4cccea255bd5095599a5bd6d4a5660bcec6d66ba))
- Cover registry crawl pipeline end to end by [@ymgyt](https://github.com/ymgyt) ([308167d2](https://github.com/ymgyt/syndicationd/commit/308167d2dedf3beb4ca565e2a43c80ba8c904e7a))
- Align unauthorized assertion with error message by [@ymgyt](https://github.com/ymgyt) ([370123f7](https://github.com/ymgyt/syndicationd/commit/370123f7b0854e918c39a286f5760fded03cefc3))

### 🧹 Refactor

- Appease clippy by [@ymgyt](https://github.com/ymgyt) ([82e05232](https://github.com/ymgyt/syndicationd/commit/82e0523279fcb2bbcd9e1e97451fd6416ceb191a))
- Migrate to sqlite by [@ymgyt](https://github.com/ymgyt) ([59a0720d](https://github.com/ymgyt/syndicationd/commit/59a0720de914e7fa1f35570ef6d86d8f27cdce5f))
- Appease clippy by [@ymgyt](https://github.com/ymgyt) ([5a7b8727](https://github.com/ymgyt/syndicationd/commit/5a7b8727daaa592692916f1ed2a4d190e58aa5b3))
- Move refresh orchestration to registry by [@ymgyt](https://github.com/ymgyt) ([44be527b](https://github.com/ymgyt/syndicationd/commit/44be527b350ad9510c6b965b48efd566a3217cb9))
- Absorb stdx and o11y by [@ymgyt](https://github.com/ymgyt) ([ed50b30f](https://github.com/ymgyt/syndicationd/commit/ed50b30f99fc1339a327433114319d1039a814a1))
- Split application components by [@ymgyt](https://github.com/ymgyt) ([34fd2a6f](https://github.com/ymgyt/syndicationd/commit/34fd2a6fb98b0877df817b6f14c518b228a9edd6))
- Split application drivers by responsibility by [@ymgyt](https://github.com/ymgyt) ([4e507c55](https://github.com/ymgyt/syndicationd/commit/4e507c5554396cd616696e38224542d4863b874f))
- Appease clippy by [@ymgyt](https://github.com/ymgyt) ([16f0b832](https://github.com/ymgyt/syndicationd/commit/16f0b8328d1cb4e5a8e016437fb0a76f771e3d6b))
- Split cli, client, and runtime crates by [@ymgyt](https://github.com/ymgyt) ([c891396e](https://github.com/ymgyt/syndicationd/commit/c891396e44a77eb00428b0a6cb8c4d01451d0391))
- Move current implementation under legacy by [@ymgyt](https://github.com/ymgyt) ([1f8e29a1](https://github.com/ymgyt/syndicationd/commit/1f8e29a190ece3b610fed7677569bacc2277b27b))
- Wire registry event runtime by [@ymgyt](https://github.com/ymgyt) ([7ec76e17](https://github.com/ymgyt/syndicationd/commit/7ec76e175ce5033b9cf15a597f6f79bdd410c20d))
- Checkpoint event runtime migration by [@ymgyt](https://github.com/ymgyt) ([cd6eed7e](https://github.com/ymgyt/syndicationd/commit/cd6eed7e6bcfe087083b926795a31d93084a3bda))
- Appease clippy by [@ymgyt](https://github.com/ymgyt) ([9b2dfb5c](https://github.com/ymgyt/syndicationd/commit/9b2dfb5ca289146474041387a9e79b3cd9cde052))
- Import tracing macros by [@ymgyt](https://github.com/ymgyt) ([fb321a08](https://github.com/ymgyt/syndicationd/commit/fb321a0855d31cfb255472e2e5ff3b36ca18aa74))
- Simplify registry event runtime by [@ymgyt](https://github.com/ymgyt) ([1faf6ea1](https://github.com/ymgyt/syndicationd/commit/1faf6ea1781ac012b52961f719518081192da893))
- Derive daemon database argument by [@ymgyt](https://github.com/ymgyt) ([5bf9033a](https://github.com/ymgyt/syndicationd/commit/5bf9033a01e0d7a32ecb0380000e6456a51b8102))
- Remove legacy registry surfaces by [@ymgyt](https://github.com/ymgyt) ([75a6bb53](https://github.com/ymgyt/syndicationd/commit/75a6bb5304dbee602b49f5830a55c97a61f093a5))
- Simplify daemon launch configuration by [@ymgyt](https://github.com/ymgyt) ([bfb8a807](https://github.com/ymgyt/syndicationd/commit/bfb8a80752b763c8e8bb4d15f5dcbc683a81c260))
- Resolve placement during construction by [@ymgyt](https://github.com/ymgyt) ([4b2223af](https://github.com/ymgyt/syndicationd/commit/4b2223af5553c858f91548820c79d52a16f254ee))
- Process events transactionally by [@ymgyt](https://github.com/ymgyt) ([a7afebd9](https://github.com/ymgyt/syndicationd/commit/a7afebd9072ed25eeb8e71302095861903584267))
- Process events in transactions by [@ymgyt](https://github.com/ymgyt) ([e863064e](https://github.com/ymgyt/syndicationd/commit/e863064ea26376ca5c5841ba1f7e44dfc9d018ec))
- Simplify api service names by [@ymgyt](https://github.com/ymgyt) ([c578ba2c](https://github.com/ymgyt/syndicationd/commit/c578ba2cd69ea136d3d3b3185b152925e1296958))
- Centralize event processing setup by [@ymgyt](https://github.com/ymgyt) ([909a7ff6](https://github.com/ymgyt/syndicationd/commit/909a7ff6d2e0c12c5d8e511fdbf0331a8db4a3a9))
- Clarify keymap v2 responsibilities by [@ymgyt](https://github.com/ymgyt) ([f79afe79](https://github.com/ymgyt/syndicationd/commit/f79afe79d440052cd61b624dca54b61a41e7fa59))
- Align crawl policy and endpoint schema by [@ymgyt](https://github.com/ymgyt) ([6642c1af](https://github.com/ymgyt/syndicationd/commit/6642c1af75747fad1ebf097a3878a74b0d693e9b))
- Streamline subscription requests by [@ymgyt](https://github.com/ymgyt) ([e5bf185c](https://github.com/ymgyt/syndicationd/commit/e5bf185c679bb6e72aac3367366491a7e846cb0a))
- Route subscriptions through subscriber scope by [@ymgyt](https://github.com/ymgyt) ([521df9eb](https://github.com/ymgyt/syndicationd/commit/521df9eb1fc1c2dc242e4df9617ddda8d0691700))
- Derive crawl targets from endpoint subscriptions by [@ymgyt](https://github.com/ymgyt) ([21a754d0](https://github.com/ymgyt/syndicationd/commit/21a754d0cd8083ab04912c367d7c2bdd94af02f9))
- Finalize keymap runtime by [@ymgyt](https://github.com/ymgyt) ([91e80f88](https://github.com/ymgyt/syndicationd/commit/91e80f88f3ad58e5c275083860cf76d3495e315d))
- Prepare TUI workflow testing by [@ymgyt](https://github.com/ymgyt) ([c108939c](https://github.com/ymgyt/syndicationd/commit/c108939cb26106caf4e812b0294d33537769174d))
- Isolate terminal application boundaries by [@ymgyt](https://github.com/ymgyt) ([5f91cb21](https://github.com/ymgyt/syndicationd/commit/5f91cb211a26eeb4edb0618736c9005b3088f073))
- Appease clippy by [@ymgyt](https://github.com/ymgyt) ([0652808b](https://github.com/ymgyt/syndicationd/commit/0652808bfe39e7714a84d0ad8b14db8da5008df3))
- Complete event and db layer cleanup by [@ymgyt](https://github.com/ymgyt) ([645b2704](https://github.com/ymgyt/syndicationd/commit/645b270417fda41f63fd847a5381274fbdb1b007))
- Simplify registry event pipeline by [@ymgyt](https://github.com/ymgyt) ([c669a5df](https://github.com/ymgyt/syndicationd/commit/c669a5df81d8c27bf44b5d4341637fac837bac53))
- Route subscriptions through command handlers by [@ymgyt](https://github.com/ymgyt) ([2637ec94](https://github.com/ymgyt/syndicationd/commit/2637ec94d80a5892f0b4285a27405c1b7be10d02))
- Align registry naming and type docs by [@ymgyt](https://github.com/ymgyt) ([9a09223e](https://github.com/ymgyt/syndicationd/commit/9a09223e756fbfe5b77f8548dfe45ad8e257c6f3))
- Add crawl scheduler dispatch scaffolding by [@ymgyt](https://github.com/ymgyt) ([e941f85c](https://github.com/ymgyt/syndicationd/commit/e941f85c32018ceb1a2184177cd4b4e82be45cdc))
- Add scheduler reconciliation scaffold by [@ymgyt](https://github.com/ymgyt) ([70bc80a6](https://github.com/ymgyt/syndicationd/commit/70bc80a643d6fad3315a635826cc3314286a8a59))
- Rework crawl scheduling runtime by [@ymgyt](https://github.com/ymgyt) ([c9896983](https://github.com/ymgyt/syndicationd/commit/c9896983a710e2d58e2ecee66dbcd5c3176a7dce))
- Restructure event pipeline and crawl dispatch by [@ymgyt](https://github.com/ymgyt) ([55dff2e0](https://github.com/ymgyt/syndicationd/commit/55dff2e0a37abd90add5bcc11879011973e934a9))
- Extract page limit helper by [@ymgyt](https://github.com/ymgyt) ([6a57ef44](https://github.com/ymgyt/syndicationd/commit/6a57ef446b599e75a595f24d303a47a97d84c1bd))
- Standardize registry module layout by [@ymgyt](https://github.com/ymgyt) ([18f8ab26](https://github.com/ymgyt/syndicationd/commit/18f8ab26b9f0fa20df24d566427102f7759d02a8))
- Standardize session module layout by [@ymgyt](https://github.com/ymgyt) ([6b555cea](https://github.com/ymgyt/syndicationd/commit/6b555ceaa9bf023ca470f279b421f03539a927e9))
- Organize terminal startup flow by [@ymgyt](https://github.com/ymgyt) ([4f20eb34](https://github.com/ymgyt/syndicationd/commit/4f20eb34f6f430e5328d70e53cb9c842525b0a21))
- Simplify feed api startup by [@ymgyt](https://github.com/ymgyt) ([84d85bd3](https://github.com/ymgyt/syndicationd/commit/84d85bd32a4cf493ea92c1c6be7bd6854c04904e))
- Model feed event subscription state by [@ymgyt](https://github.com/ymgyt) ([6b26566a](https://github.com/ymgyt/syndicationd/commit/6b26566a071c1e9f6214e4ac760a3633f86dafe1))
- Remove term subcommand by [@ymgyt](https://github.com/ymgyt) ([13e9fff8](https://github.com/ymgyt/syndicationd/commit/13e9fff875264688b49526f94d8e03f9492ae818))
- Rebuild application lifecycle and drivers by [@ymgyt](https://github.com/ymgyt) ([449fbfba](https://github.com/ymgyt/syndicationd/commit/449fbfba0d409516794fa4a2e3e193ae5e7064de))
- Render after every event by [@ymgyt](https://github.com/ymgyt) ([58063b83](https://github.com/ymgyt/syndicationd/commit/58063b832a9ea1c0bd4d2789f66cfbca89c9cca2))
- Derive pending key discard from keymap layers by [@ymgyt](https://github.com/ymgyt) ([9cf57426](https://github.com/ymgyt/syndicationd/commit/9cf57426212b114dcce8b7216661500ca850bd70))
- Remove legacy entries surface and feed url filter by [@ymgyt](https://github.com/ymgyt) ([43ac3c70](https://github.com/ymgyt/syndicationd/commit/43ac3c70ac0b48c9a9fbb5a670637f8bde9843e1))
- Remove broken feed refresh machinery by [@ymgyt](https://github.com/ymgyt) ([1563ac94](https://github.com/ymgyt/syndicationd/commit/1563ac94be817bd459706cccf037bdf7049b4560))
- Converge subscriptions on feedRegistry root by [@ymgyt](https://github.com/ymgyt) ([0e4754f0](https://github.com/ymgyt/syndicationd/commit/0e4754f05caec6e0ec7ee0e58eae4ce15a69a62e))
- Bootstrap with subscription and timeline queries by [@ymgyt](https://github.com/ymgyt) ([b4c60164](https://github.com/ymgyt/syndicationd/commit/b4c601645384b6fa790ffb97f5ccf8bff747069d))
- Drop dead entries reload from subscription apply by [@ymgyt](https://github.com/ymgyt) ([91dad0ba](https://github.com/ymgyt/syndicationd/commit/91dad0ba20921b370b834d708de6bc622b717b8f))
- Reload subscriptions with r on feeds tab by [@ymgyt](https://github.com/ymgyt) ([dfea2c44](https://github.com/ymgyt/syndicationd/commit/dfea2c4414d7fa87d11caec94b7a2aa1b3abe7db))
- Redesign schema as declared and observed state by [@ymgyt](https://github.com/ymgyt) ([dc830b4d](https://github.com/ymgyt/syndicationd/commit/dc830b4de61d1666c019d90ba2c812cbbb274139))
- Remove feed view sync machinery by [@ymgyt](https://github.com/ymgyt) ([8d8ad33d](https://github.com/ymgyt/syndicationd/commit/8d8ad33dbacb4349eda404468b7dc957fef302d7))
- Own parsed entry model by [@ymgyt](https://github.com/ymgyt) ([d10bccdd](https://github.com/ymgyt/syndicationd/commit/d10bccdd3d06cb1178dc64a219a6f728b6140ede))
- Rename database capability traits by [@ymgyt](https://github.com/ymgyt) ([9892f7d3](https://github.com/ymgyt/syndicationd/commit/9892f7d33d0b4d21eeada43cdaa8b981fde3e38d))
- Make entry a first-class module by [@ymgyt](https://github.com/ymgyt) ([2d1350c6](https://github.com/ymgyt/syndicationd/commit/2d1350c62b932ec3cf0e3646887ddb99dce73187))
- Remove terminal stop logging by [@ymgyt](https://github.com/ymgyt) ([33bc54ca](https://github.com/ymgyt/syndicationd/commit/33bc54cafcb39a019f80ea81889f0aa0e7de0788))
- Clarify driver operation and event flow by [@ymgyt](https://github.com/ymgyt) ([3cdd35b9](https://github.com/ymgyt/syndicationd/commit/3cdd35b92d17696e7d25c8971ed7fb9e077b0e97))
- Expose OpenTelemetry extension traits by [@ymgyt](https://github.com/ymgyt) ([f9baac20](https://github.com/ymgyt/syndicationd/commit/f9baac20f65805962e9eb8ecf9839ec00b757e80))
- Redesign terminal application and client boundaries by [@ymgyt](https://github.com/ymgyt) ([54b4018b](https://github.com/ymgyt/syndicationd/commit/54b4018b5f77612836443eba7813f7d92a1e8f5f))
- Remove dry-run startup probe by [@ymgyt](https://github.com/ymgyt) ([d75c7849](https://github.com/ymgyt/syndicationd/commit/d75c7849d72c38ceca05462c1d85bcf6eae43f9d))
- Separate migration errors by [@ymgyt](https://github.com/ymgyt) ([0d05a68d](https://github.com/ymgyt/syndicationd/commit/0d05a68d72bc213c4dfdfdbc136750710c5c8e20))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.3.2...v0.4.0-rc.1


## [synd-term-v0.3.2] - 2025-01-25

### 📡 Features

- Add dracula and eldritch themes (#225) by [@tangowithfoxtrot](https://github.com/tangowithfoxtrot) ([c4e6a333](https://github.com/ymgyt/syndicationd/commit/c4e6a33354514428d311267d9eb0b9a370ebe59c))

### ⚙️ Miscellaneous Tasks

- Reorganize release tasks by [@ymgyt](https://github.com/ymgyt) ([e920156a](https://github.com/ymgyt/syndicationd/commit/e920156aab15b5a839cadf5e56af2883c9c5708f))
- Refactor justfile by [@ymgyt](https://github.com/ymgyt) ([56b21266](https://github.com/ymgyt/syndicationd/commit/56b21266d79131d63e2cb6ce4136f75f8865f262))
- Standardize graphql colocation by [@ymgyt](https://github.com/ymgyt) ([ceb479b8](https://github.com/ymgyt/syndicationd/commit/ceb479b8adbba74f1fb6999b1194e2de21b64f6a))
- Bump html2text from 0.12.6 to 0.13.2 by [@ymgyt](https://github.com/ymgyt) ([6db8cb89](https://github.com/ymgyt/syndicationd/commit/6db8cb8995c8b72c11459ee619716f873d6d1912))
- Bump ratatui from 0.28.1 to 0.29.0 by [@ymgyt](https://github.com/ymgyt) ([98c88658](https://github.com/ymgyt/syndicationd/commit/98c886582261d8f14eb35690a995e50190817e05))
- Bump nom from 7.1.3 to 8.0.0-alpha2 by [@ymgyt](https://github.com/ymgyt) ([c2a313a2](https://github.com/ymgyt/syndicationd/commit/c2a313a20a6157bc1e776825374961926fa33ad9))

### 📚 Documentation

- Reorganize mdbook dir by [@ymgyt](https://github.com/ymgyt) ([7fff65bb](https://github.com/ymgyt/syndicationd/commit/7fff65bbc592483cb9d828135d09fdbfc5df9713))

### 🔧 Testing

- Cover new themes by [@ymgyt](https://github.com/ymgyt) ([b48cc783](https://github.com/ymgyt/syndicationd/commit/b48cc7835b10255fa82479c9f94ec2d4054a7ea6))

### 🧹 Refactor

- Migrate humantime to stdx by [@ymgyt](https://github.com/ymgyt) ([5908219d](https://github.com/ymgyt/syndicationd/commit/5908219dc47c40969e3306063e673618fe52e658))
- Migrate deserialization method to stdx by [@ymgyt](https://github.com/ymgyt) ([cd8c7751](https://github.com/ymgyt/syndicationd/commit/cd8c775198bb21793fedd0f5ec0a8d2930b5596c))
- Migrate conf Entry to stdx by [@ymgyt](https://github.com/ymgyt) ([fe75d6f0](https://github.com/ymgyt/syndicationd/commit/fe75d6f06b8c4720d359942b77fe980b7754d82b))
- Migrate filesystem trait to stdx by [@ymgyt](https://github.com/ymgyt) ([03fca010](https://github.com/ymgyt/syndicationd/commit/03fca010209a41fcf1deca5ae7a0a5f7d61bbe63))
- Rename args module to cli by [@ymgyt](https://github.com/ymgyt) ([8a4c3487](https://github.com/ymgyt/syndicationd/commit/8a4c3487c8246cf6f67ece0b637a78ecdc9ee17e))
- Appease clippy by [@ymgyt](https://github.com/ymgyt) ([c3756d13](https://github.com/ymgyt/syndicationd/commit/c3756d13c8414550ba7e27c90f3f6487857a46d0))
- Appease clippy by [@ymgyt](https://github.com/ymgyt) ([d525dbea](https://github.com/ymgyt/syndicationd/commit/d525dbeaa520a4bdc40fe643b1f0583aa328a20a))
- Appease clippy by [@ymgyt](https://github.com/ymgyt) ([21248fbd](https://github.com/ymgyt/syndicationd/commit/21248fbd7cc03b591037b4fb1a42f9c7453f5e56))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.3.1...synd-term-v0.3.2


* @tangowithfoxtrot made their first contribution in #225

## [synd-term-v0.3.1] - 2024-08-31

### 📡 Features

- Add import command (#132) by [@ymgyt](https://github.com/ymgyt) ([3513a253](https://github.com/ymgyt/syndicationd/commit/3513a2530b7ff3ecf8bc75ef1f03a49f34a01a81))
- Support configuration file by [@ymgyt](https://github.com/ymgyt) ([86d7b246](https://github.com/ymgyt/syndicationd/commit/86d7b246276193d8790cb3bc0a092789919c7e19))
- Set timeout for the github client by [@ymgyt](https://github.com/ymgyt) ([96e47621](https://github.com/ymgyt/syndicationd/commit/96e47621d1f55977b97565c96fd969a2e92990f9))

### ⚙️ Miscellaneous Tasks

- Add benchmark for rendering (#93) by [@ymgyt](https://github.com/ymgyt) ([0a1d4d3f](https://github.com/ymgyt/syndicationd/commit/0a1d4d3f578c1a531c41b7db381f6b00da684cd5))
- Support flamegraph by [@ymgyt](https://github.com/ymgyt) ([e09f315e](https://github.com/ymgyt/syndicationd/commit/e09f315ec529334a2f6257ff55bcb32df1fddd8d))
- Mk etc dir by [@ymgyt](https://github.com/ymgyt) ([fd209bc8](https://github.com/ymgyt/syndicationd/commit/fd209bc8695b0334fe97829afcb2a959fba9e24f))
- Upgrade tui-big-text from 0.4.5 to 0.5.3 by [@ymgyt](https://github.com/ymgyt) ([23de054b](https://github.com/ymgyt/syndicationd/commit/23de054b02a66f1910b60f18e44b88d2c8c308b0))
- Use HighlightSpacing::Alway when rendering table by [@ymgyt](https://github.com/ymgyt) ([fe357e9c](https://github.com/ymgyt/syndicationd/commit/fe357e9c5666762e3bce060c5ea4f79c69dfc61c))
- Trim the output of config init by [@ymgyt](https://github.com/ymgyt) ([e1c3e048](https://github.com/ymgyt/syndicationd/commit/e1c3e048ce3159bb9f95c63664eb805706bed91f))
- Update cli help by [@ymgyt](https://github.com/ymgyt) ([35981ef0](https://github.com/ymgyt/syndicationd/commit/35981ef05537b5eb1ff82d73a42ab54bc7aeedb9))
- Handle command not found error by [@ymgyt](https://github.com/ymgyt) ([b6b914c0](https://github.com/ymgyt/syndicationd/commit/b6b914c03077a378c2c0e77a4ac7b6154aa3a152))
- Add pat validation during github client construction by [@ymgyt](https://github.com/ymgyt) ([f75ddf9a](https://github.com/ymgyt/syndicationd/commit/f75ddf9a22a2c1dcf41b2dcdcf1b48131d5d79ea))
- Add validation method to ConfigResolver by [@ymgyt](https://github.com/ymgyt) ([9152458b](https://github.com/ymgyt/syndicationd/commit/9152458b7514139ae8644403ad7b94552be5d42e))
- Change unreferenced value in insta test by [@ymgyt](https://github.com/ymgyt) ([4fa07c39](https://github.com/ymgyt/syndicationd/commit/4fa07c39c7d153f7d3d6f8b1d6e855baa0a1d927))

### 🎨 Styling

- Show help key for brose entry command by [@ymgyt](https://github.com/ymgyt) ([ba7bb2f2](https://github.com/ymgyt/syndicationd/commit/ba7bb2f2850e2744b3afc28addeb4fb4894162d8))

### 📚 Documentation

- Add description for import command by [@ymgyt](https://github.com/ymgyt) ([f77dff7c](https://github.com/ymgyt/syndicationd/commit/f77dff7cca03a0a09949ca1f9925a89485a6a48f))

### 🔧 Testing

- Relax interval assertion in import test by [@ymgyt](https://github.com/ymgyt) ([822d6f4c](https://github.com/ymgyt/syndicationd/commit/822d6f4cdf5a94df130c6bd2fc3ff437e4302750))

### 🧹 Refactor

- Abstract file system by [@ymgyt](https://github.com/ymgyt) ([9cbef556](https://github.com/ymgyt/syndicationd/commit/9cbef556b42abff85e057c6a37292198279e58e6))
- Add create operations to fs trait by [@ymgyt](https://github.com/ymgyt) ([db46ab02](https://github.com/ymgyt/syndicationd/commit/db46ab02c1ff299a9ee349e62298c51392edddca))
- Remove duplicat code in cache's load and persis methods by [@ymgyt](https://github.com/ymgyt) ([e9445da8](https://github.com/ymgyt/syndicationd/commit/e9445da874c316f97754b37a23486dacda3b4875))
- Abstract interactor (#121) by [@ymgyt](https://github.com/ymgyt) ([f0283f8d](https://github.com/ymgyt/syndicationd/commit/f0283f8d0dae98f58ed2023bd8863ec307f3eed3))
- Add custom table widget to remove duplicate code by [@ymgyt](https://github.com/ymgyt) ([5a2953c6](https://github.com/ymgyt/syndicationd/commit/5a2953c6ee9fa0292779b86a6a50430cd787cc9c))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.3.0...synd-term-v0.3.1


## [synd-term-v0.3.0] - 2024-07-11

### 📡 Features

- Handle github secondary rate limit error by [@ymgyt](https://github.com/ymgyt) ([9f690c67](https://github.com/ymgyt/syndicationd/commit/9f690c675870b359150738c66029224f5c94b52f))
- Limit the number of concurrent executions of job futures by [@ymgyt](https://github.com/ymgyt) ([62b51bb0](https://github.com/ymgyt/syndicationd/commit/62b51bb041b151593cc634ca30114d356243ceff))

### 🧹 Refactor

- Use the filter of FilterableVec as the primary source by [@ymgyt](https://github.com/ymgyt) ([f1739fe8](https://github.com/ymgyt/syndicationd/commit/f1739fe811749466038070d242a2913a7524baaf))
- Use macro to impl newtype by [@ymgyt](https://github.com/ymgyt) ([17797a5c](https://github.com/ymgyt/syndicationd/commit/17797a5c25b2c30d4b20dfede7cf331fd1ad8c6b))
- Split background futures to another jobs by [@ymgyt](https://github.com/ymgyt) ([995f143b](https://github.com/ymgyt/syndicationd/commit/995f143bce4237bfceacad816b86522e59d268a2))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.3.0-rc.2...synd-term-v0.3.0


## [synd-term-v0.3.0-rc.2] - 2024-07-07

### 🐛 Bug Fixes

- Apply initial filter by [@ymgyt](https://github.com/ymgyt) ([47ed7abf](https://github.com/ymgyt/syndicationd/commit/47ed7abf59ca8d11abff337dc1e141b83752595e))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.3.0-rc.1...synd-term-v0.3.0-rc.2


## [synd-term-v0.3.0-rc.1] - 2024-07-07

### 📡 Features

- Use local timezone when displaying time (#71) by [@ymgyt](https://github.com/ymgyt) ([36212b4b](https://github.com/ymgyt/syndicationd/commit/36212b4b1d00855b0b4206a45714cc42359dfb8e))
- Handle terminal focus event by [@ymgyt](https://github.com/ymgyt) ([27f02a36](https://github.com/ymgyt/syndicationd/commit/27f02a36aead07994fb495eb4305d32e3fd0bdd4))
- Support github notification (#77) by [@ymgyt](https://github.com/ymgyt) ([b3fc9958](https://github.com/ymgyt/syndicationd/commit/b3fc9958e739df57fb212ca0f986cb9cc25af333))
- Category filtering for github notifications (#78) by [@ymgyt](https://github.com/ymgyt) ([e67b0502](https://github.com/ymgyt/syndicationd/commit/e67b0502e8039844f2dc579af84f9caa9ede8bfe))
- Display labels for github issues and PRs by [@ymgyt](https://github.com/ymgyt) ([1cd28d0c](https://github.com/ymgyt/syndicationd/commit/1cd28d0c06305d44820e19ecf4ae9bc08a54c1c0))
- Add github filter conditions (#85) by [@ymgyt](https://github.com/ymgyt) ([a1135c7e](https://github.com/ymgyt/syndicationd/commit/a1135c7e94f71b74cbb98b6ce3e26c67d1c029f4))
- Add mark_as_done_all command by [@ymgyt](https://github.com/ymgyt) ([4633d73c](https://github.com/ymgyt/syndicationd/commit/4633d73caaef414cc830d445b1af4431763ac389))
- Persist github notifications filter options by [@ymgyt](https://github.com/ymgyt) ([28ba85ee](https://github.com/ymgyt/syndicationd/commit/28ba85ee2957e3a513b8814e46fb15b741f0e67a))
- Handle github unauthorized error by [@ymgyt](https://github.com/ymgyt) ([0f9acbb5](https://github.com/ymgyt/syndicationd/commit/0f9acbb536c1241534f8a67306a589db819bf638))

### 🐛 Bug Fixes

- Make tab width dynamic by [@ymgyt](https://github.com/ymgyt) ([9679d7da](https://github.com/ymgyt/syndicationd/commit/9679d7da3e6315bebb46a70a13a3ab4e8ce24fa5))
- Fix graphql schema path by [@ymgyt](https://github.com/ymgyt) ([f1abe4b7](https://github.com/ymgyt/syndicationd/commit/f1abe4b76294c4a6111fa440c3c77fd3e55c5d23))

### ⚙️ Miscellaneous Tasks

- Update ratatui from 0.26.3 to 0.27.0 by [@ymgyt](https://github.com/ymgyt) ([968c3256](https://github.com/ymgyt/syndicationd/commit/968c32564b6dd2882da413d88320077989464f8c))
- Include graphql files by [@ymgyt](https://github.com/ymgyt) ([453d630e](https://github.com/ymgyt/syndicationd/commit/453d630e07b5f849b0f981619e0403977b1887c3))
- Copy github graphql schema by [@ymgyt](https://github.com/ymgyt) ([ce90f72b](https://github.com/ymgyt/syndicationd/commit/ce90f72b9acf461d6aeadfe1413602dcc851a2ef))

### 🎨 Styling

- Use italic modifier for gh notification filters by [@ymgyt](https://github.com/ymgyt) ([ead785ac](https://github.com/ymgyt/syndicationd/commit/ead785acfbb32d766f238b847b87b0a4a8ed6654))

### 🔧 Testing

- Add test case for focus gained event by [@ymgyt](https://github.com/ymgyt) ([423d201f](https://github.com/ymgyt/syndicationd/commit/423d201f35b94217c0ff9a68e1442f261dcfb2e9))
- Add gql fixtures to gh notifications test by [@ymgyt](https://github.com/ymgyt) ([1616c526](https://github.com/ymgyt/syndicationd/commit/1616c526e076547648661705fec2fafd66090e88))
- Add test case for filtering gh notifications by [@ymgyt](https://github.com/ymgyt) ([cf45254d](https://github.com/ymgyt/syndicationd/commit/cf45254d7db5080cf1753bb33cfa2c9d91c10cce))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.2.6...synd-term-v0.3.0-rc.1


## [synd-term-v0.2.6] - 2024-06-18

### 📡 Features

- Simplify ui by [@ymgyt](https://github.com/ymgyt) ([b2fa928d](https://github.com/ymgyt/syndicationd/commit/b2fa928de37cf0c431d63e2e2f2b17e6dee19250))
- Show entry detail by [@ymgyt](https://github.com/ymgyt) ([e9162afa](https://github.com/ymgyt/syndicationd/commit/e9162afa903277d751ed8abf964a275668f6096c))
- Inform latest release by [@ymgyt](https://github.com/ymgyt) ([a65eb66c](https://github.com/ymgyt/syndicationd/commit/a65eb66ccedd737f95dcea78c0e12770016ffade))
- Match feed url by [@ymgyt](https://github.com/ymgyt) ([d077a320](https://github.com/ymgyt/syndicationd/commit/d077a320226e26a7cba928c619822c8509b7548b))

### ⚙️ Miscellaneous Tasks

- Update ratatui from 0.26.1 to 0.26.3 by [@ymgyt](https://github.com/ymgyt) ([5260fc8b](https://github.com/ymgyt/syndicationd/commit/5260fc8b18dcd268ec2f912f9d7ac88e7de214fd))
- Simplyfi fileter ui by [@ymgyt](https://github.com/ymgyt) ([e7815ada](https://github.com/ymgyt/syndicationd/commit/e7815adaeff96171e9f6ab825591dc12794d08a2))
- Rename homebrew formula from synd to synd-term by [@ymgyt](https://github.com/ymgyt) ([79ca0f87](https://github.com/ymgyt/syndicationd/commit/79ca0f87bb69b9aa9e093a41c5667251f86c56fc))
- Add docker description by [@ymgyt](https://github.com/ymgyt) ([a7ab8ae2](https://github.com/ymgyt/syndicationd/commit/a7ab8ae22f71b955def42a9b306b6ee1a42c8d9e))
- Rename oranda project from synd to synd-term by [@ymgyt](https://github.com/ymgyt) ([383a3592](https://github.com/ymgyt/syndicationd/commit/383a3592d764482e11f3c65b3afaea607d2b9acb))
- Use default-features instead of default_features by [@ymgyt](https://github.com/ymgyt) ([fd827f4c](https://github.com/ymgyt/syndicationd/commit/fd827f4cba9e576a9342e1115fdf7e804471d438))

### 🎨 Styling

- Remove right padding for consistency by [@ymgyt](https://github.com/ymgyt) ([79b59ca5](https://github.com/ymgyt/syndicationd/commit/79b59ca520c11041f1b68dd146751d2c91a3de32))
- Remove icon from table header for alignment by [@ymgyt](https://github.com/ymgyt) ([ef156c94](https://github.com/ymgyt/syndicationd/commit/ef156c9407a219b4fbe47dca9c067773153a4614))
- Fix search alignment by [@ymgyt](https://github.com/ymgyt) ([43f30fc6](https://github.com/ymgyt/syndicationd/commit/43f30fc6c7848e581af766320162abbfd7800ead))

### 📚 Documentation

- Use oranda mdbook component for rendering privacy policy by [@ymgyt](https://github.com/ymgyt) ([c91bc08c](https://github.com/ymgyt/syndicationd/commit/c91bc08ce4cf92629ca3438be0dabe5f97263fa9))

### 🔧 Testing

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

### 🧹 Refactor

- Remove unused code by [@ymgyt](https://github.com/ymgyt) ([bd428f17](https://github.com/ymgyt/syndicationd/commit/bd428f1724e2e97b6fbd97dad4d1e539b2e9420d))
- Avoid rendering during key event handling if possible by [@ymgyt](https://github.com/ymgyt) ([db42f5c7](https://github.com/ymgyt/syndicationd/commit/db42f5c7236495bdde39160774e8286d809a844b))
- Abstract cache access by [@ymgyt](https://github.com/ymgyt) ([32ae36e0](https://github.com/ymgyt/syndicationd/commit/32ae36e05eafcb649df3c4fcdd9dac4f33272e9c))
- Use ApplicationBuilder to construct Application by [@ymgyt](https://github.com/ymgyt) ([985edfd9](https://github.com/ymgyt/syndicationd/commit/985edfd9304e748f9f21a93b6f771b652a9ea67a))
- Use tokio_util CancellationToken instead of mpsc channel by [@ymgyt](https://github.com/ymgyt) ([3ca15bf8](https://github.com/ymgyt/syndicationd/commit/3ca15bf854882d64b51b825bbeff6db6f2c66ae0))
- Remove unused code by [@ymgyt](https://github.com/ymgyt) ([a46f99fd](https://github.com/ymgyt/syndicationd/commit/a46f99fdda7d3eed9a847bf53f2add980884d699))
- Move generated gql client code to generated dir by [@ymgyt](https://github.com/ymgyt) ([8600f559](https://github.com/ymgyt/syndicationd/commit/8600f559909ba39da7881d0f6d68dfab664fb7d4))
- Remove duplicate processing in InFlight::remove by [@ymgyt](https://github.com/ymgyt) ([0b9c8b0c](https://github.com/ymgyt/syndicationd/commit/0b9c8b0c4c40f00eccbe6592eafdd9be04b9fa4c))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.2.5...synd-term-v0.2.6


## [synd-term-v0.2.5] - 2024-05-23

### 📡 Features

- Add ferra, solarized_dark, helix themes by [@ymgyt](https://github.com/ymgyt) ([d463de09](https://github.com/ymgyt/syndicationd/commit/d463de090b91d792aed28d3d4a1e423989281a4c))

### ⚙️ Miscellaneous Tasks

- Avoid using fonts that cause issues when terminal opacity is enabled by [@ymgyt](https://github.com/ymgyt) ([13c7b8d5](https://github.com/ymgyt/syndicationd/commit/13c7b8d506cecad3f255f22809eacdee2419db2a))

### 🧹 Refactor

- Use std::ops::ControlFlow for app loop control by [@ymgyt](https://github.com/ymgyt) ([99423986](https://github.com/ymgyt/syndicationd/commit/9942398608c174c1ff41b7c18f1e3169fc857c7d))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.2.4...synd-term-v0.2.5


## [synd-term-v0.2.4] - 2024-05-06

### 📡 Features

- Show big text on login by [@ymgyt](https://github.com/ymgyt) ([d4a5b18e](https://github.com/ymgyt/syndicationd/commit/d4a5b18e7d9771a4ff5647da059f187ce0c240b6))
- Refresh google id token periodically by [@ymgyt](https://github.com/ymgyt) ([b5e0ae1f](https://github.com/ymgyt/syndicationd/commit/b5e0ae1f22f0a4c14479fe55caf11c4d4d0e6a22))
- Friendly nom parse error by [@ymgyt](https://github.com/ymgyt) ([8664e3d7](https://github.com/ymgyt/syndicationd/commit/8664e3d71ab21fd0b34515bef4efd6d9d595b11e))
- Paginate entries and feeds by [@ymgyt](https://github.com/ymgyt) ([794f65da](https://github.com/ymgyt/syndicationd/commit/794f65dabb114d7f80069b6d65813a39560ffc40))
- Make entries limit configurable by [@ymgyt](https://github.com/ymgyt) ([206bbad7](https://github.com/ymgyt/syndicationd/commit/206bbad791f5c4dc3800af8bfd190cc9ad1469e5))
- Show entries count indicator by [@ymgyt](https://github.com/ymgyt) ([fa4abc7e](https://github.com/ymgyt/syndicationd/commit/fa4abc7e0961844bede78595dbca06fd37dcbe28))
- Add unsubscribe popup by [@ymgyt](https://github.com/ymgyt) ([d7db5140](https://github.com/ymgyt/syndicationd/commit/d7db51402c940c4fce41bf9b2c9fd18b08aef25b))

### 🐛 Bug Fixes

- Filter categories duplication by [@ymgyt](https://github.com/ymgyt) ([60ec0f7a](https://github.com/ymgyt/syndicationd/commit/60ec0f7a592519404bec74006db35059e73baae7))
- Handle too small width case by [@ymgyt](https://github.com/ymgyt) ([62b5b336](https://github.com/ymgyt/syndicationd/commit/62b5b3365b341432aaf0e5fc7cf1dc970e49646c))

### ⚙️ Miscellaneous Tasks

- Change feed entries count to fetch by [@ymgyt](https://github.com/ymgyt) ([979231e9](https://github.com/ymgyt/syndicationd/commit/979231e9761bc3b4a041648155018fd7077456d6))
- Prevent selection out of index by [@ymgyt](https://github.com/ymgyt) ([1cf01601](https://github.com/ymgyt/syndicationd/commit/1cf01601325b671e62ef4398d73e4aa61c9cffbc))
- Make column order consistent by [@ymgyt](https://github.com/ymgyt) ([fecafd98](https://github.com/ymgyt/syndicationd/commit/fecafd988b937d57a7a62cc8c1abc6dd903e4141))
- Logging feeds that failed to fetch by [@ymgyt](https://github.com/ymgyt) ([425548cb](https://github.com/ymgyt/syndicationd/commit/425548cbab0728ac54d28c30e5e76ba384e50c78))

### 🧹 Refactor

- Clippy by [@ymgyt](https://github.com/ymgyt) ([ddc8fa66](https://github.com/ymgyt/syndicationd/commit/ddc8fa66d5d6d7b4dcb3892a147bf90552080cbf))
- Use bitflags to manage app flags by [@ymgyt](https://github.com/ymgyt) ([aa2d6c49](https://github.com/ymgyt/syndicationd/commit/aa2d6c491c591e4f966c87d2489395f6f96cf3fb))
- Count keymap capacity by [@ymgyt](https://github.com/ymgyt) ([466368f4](https://github.com/ymgyt/syndicationd/commit/466368f46b65b325959e740358d816fb9d602dd7))
- Rename parse module to service by [@ymgyt](https://github.com/ymgyt) ([256542d9](https://github.com/ymgyt/syndicationd/commit/256542d9955811eac0c26b350f528cce1106dd50))
- Reduce visibility by [@ymgyt](https://github.com/ymgyt) ([08df3e55](https://github.com/ymgyt/syndicationd/commit/08df3e55dd3deac1ef7f7445a2cedaa9b8d20bdb))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.2.3...synd-term-v0.2.4


## [synd-term-v0.2.3] - 2024-04-29

### 📡 Features

- Add search by [@ymgyt](https://github.com/ymgyt) ([ad68a603](https://github.com/ymgyt/syndicationd/commit/ad68a603161f3ed0d0722eccb010851b82b6276e))

### ⚙️ Miscellaneous Tasks

- Change oranda project name from synd-term to synd by [@ymgyt](https://github.com/ymgyt) ([802892ad](https://github.com/ymgyt/syndicationd/commit/802892ad8351c546e5a80b6edeeba981a515a526))
- Rename clear command to clean by [@ymgyt](https://github.com/ymgyt) ([767adc34](https://github.com/ymgyt/syndicationd/commit/767adc34460a06dc8771fba55f7b2affd2da994c))

### 🔧 Testing

- Add matcher test by [@ymgyt](https://github.com/ymgyt) ([f1dc9564](https://github.com/ymgyt/syndicationd/commit/f1dc9564a371fee96b0b8a742eeb87cf8474397e))

### 🧹 Refactor

- Use FeedUrl instead of String by [@ymgyt](https://github.com/ymgyt) ([7503ae0e](https://github.com/ymgyt/syndicationd/commit/7503ae0e8c72061ce1f1bcb01112b55c744beac6))
- Make tests module consistent by [@ymgyt](https://github.com/ymgyt) ([a0c2c530](https://github.com/ymgyt/syndicationd/commit/a0c2c5300372f9a7d9e7f96c3a2bda5a620e755f))
- Rename prompt to status line by [@ymgyt](https://github.com/ymgyt) ([6e3c8850](https://github.com/ymgyt/syndicationd/commit/6e3c885057729f2e08c524224584c097f72a3f59))
- Rename filter method by [@ymgyt](https://github.com/ymgyt) ([4cc525fc](https://github.com/ymgyt/syndicationd/commit/4cc525fc6d5644783c9f93cbd60ffc65a0a8cb52))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.2.2...synd-term-v0.2.3


## [synd-term-v0.2.2] - 2024-04-18

### 🐛 Bug Fixes

- Use selected_feed to render feed detail by [@ymgyt](https://github.com/ymgyt) ([404cc4cf](https://github.com/ymgyt/syndicationd/commit/404cc4cf6f52129a9a32bb58a23a3e9eb1e98efb))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.2.1...synd-term-v0.2.2


## [synd-term-v0.2.1] - 2024-04-17

### 🐛 Bug Fixes

- Rollback ratatui from 0.26.2 to 0.26.1 by [@ymgyt](https://github.com/ymgyt) ([75b6db7f](https://github.com/ymgyt/syndicationd/commit/75b6db7ff5f237dba68fdb0480c1af4edede7dbd))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.2.0...synd-term-v0.2.1


## [synd-term-v0.2.0] - 2024-04-17

### 📡 Features

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

### 🐛 Bug Fixes

- Remove unsubscribed category from filter by [@ymgyt](https://github.com/ymgyt) ([6f5b2cb4](https://github.com/ymgyt/syndicationd/commit/6f5b2cb40dc74de3a833bba8f3ec25b52adfcf3b))

### ⚙️ Miscellaneous Tasks

- Add pacman to oranda install section by [@ymgyt](https://github.com/ymgyt) ([873254cd](https://github.com/ymgyt/syndicationd/commit/873254cd5fa9c8667e8043b3d1462faeb0ff0c0a))
- Fix check command typo by [@ymgyt](https://github.com/ymgyt) ([59ed1b83](https://github.com/ymgyt/syndicationd/commit/59ed1b83f911447144e648bbab2657c1d8f3bf59))
- Make table column capital consistent by [@ymgyt](https://github.com/ymgyt) ([a9b7eff9](https://github.com/ymgyt/syndicationd/commit/a9b7eff978dfc44f1bc0511bb4c465f21939ec83))
- Capitalize feed detail columns by [@ymgyt](https://github.com/ymgyt) ([ed9f0668](https://github.com/ymgyt/syndicationd/commit/ed9f0668d4d970c9181712336d9863f222aaaa4c))
- Remove feed prefix from feed detail component by [@ymgyt](https://github.com/ymgyt) ([09a0a410](https://github.com/ymgyt/syndicationd/commit/09a0a41082be9ad26340e0a560793bb697cdf21f))
- Increase fetched entries count by [@ymgyt](https://github.com/ymgyt) ([e5177160](https://github.com/ymgyt/syndicationd/commit/e5177160ace15c54a17c8bad070a1767a4fb76b8))
- Fix typo by [@ymgyt](https://github.com/ymgyt) ([a70475ec](https://github.com/ymgyt/syndicationd/commit/a70475ec3a3dc284b1a209ace81bd29dcaaee00d))
- Change category filter keymap by [@ymgyt](https://github.com/ymgyt) ([8a736ed4](https://github.com/ymgyt/syndicationd/commit/8a736ed46d970ec5e245bff1a8fa3ac7adaad21f))
- Create symlink to categories.toml by [@ymgyt](https://github.com/ymgyt) ([c990584c](https://github.com/ymgyt/syndicationd/commit/c990584c4865242a6fe97d72b63c3a2ba1b36616))

### 🔧 Testing

- Fix integration by [@ymgyt](https://github.com/ymgyt) ([0348ec21](https://github.com/ymgyt/syndicationd/commit/0348ec21b1605371c3532c648a29aca92680f7f0))

### 🧹 Refactor

- Add helix like keymaps by [@ymgyt](https://github.com/ymgyt) ([257beaad](https://github.com/ymgyt/syndicationd/commit/257beaad941844b931140d8b967812cf41ce2e22))
- Resolve key events using an arrary of keymaps by [@ymgyt](https://github.com/ymgyt) ([311e3848](https://github.com/ymgyt/syndicationd/commit/311e38480f2959535cf9c7302cf335155fa15d6c))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.10...synd-term-v0.2.0


## [synd-term-v0.1.10] - 2024-03-19

### 📡 Features

- Make the order of keymap help consistent by [@ymgyt](https://github.com/ymgyt) ([76d385e3](https://github.com/ymgyt/syndicationd/commit/76d385e31d3f46513b4c5c39e6166f72874f16be))
- Change time format delimiter by [@ymgyt](https://github.com/ymgyt) ([b0768bc1](https://github.com/ymgyt/syndicationd/commit/b0768bc11980f9a620b6857639a33a0c01cc3fe8))
- Change entry column name by [@ymgyt](https://github.com/ymgyt) ([fd15bf63](https://github.com/ymgyt/syndicationd/commit/fd15bf633dab04159b69754901b6962cdc3f6d38))

### ⚙️ Miscellaneous Tasks

- Set log level for the credential restore process to debug by [@ymgyt](https://github.com/ymgyt) ([30dc7a2f](https://github.com/ymgyt/syndicationd/commit/30dc7a2f71e34ce431435acdc21c170c86c677b0))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.9...synd-term-v0.1.10


## [synd-term-v0.1.9] - 2024-03-18

### 📡 Features

- Fallback latest entries published date by [@ymgyt](https://github.com/ymgyt) ([98b57a10](https://github.com/ymgyt/syndicationd/commit/98b57a108e9b137b47e76f9e88bfa900db46ff8a))
- Handle ctrl-c by [@ymgyt](https://github.com/ymgyt) ([ef2842e2](https://github.com/ymgyt/syndicationd/commit/ef2842e2cdd1bf4e0468e69e5cdea06869fa17b3))
- Make the space policy consistent by [@ymgyt](https://github.com/ymgyt) ([0a3d9dfb](https://github.com/ymgyt/syndicationd/commit/0a3d9dfb8c91f8ea2875dd99a423ae3189f17e56))
- Change detail border type by [@ymgyt](https://github.com/ymgyt) ([099c8524](https://github.com/ymgyt/syndicationd/commit/099c8524fa8a2af1fefd1bab3f8cd6cb91edee42))
- Change feed meta widget from list to table by [@ymgyt](https://github.com/ymgyt) ([f583e2f2](https://github.com/ymgyt/syndicationd/commit/f583e2f27dbe11c0f6348ab43bd918877d719d6e))

### 🐛 Bug Fixes

- Remove debug logging by [@ymgyt](https://github.com/ymgyt) ([066b3cc0](https://github.com/ymgyt/syndicationd/commit/066b3cc04490d15618a1503098fe4e1aae411198))

### 🧹 Refactor

- Rename jwt_decoder to jwt_service by [@ymgyt](https://github.com/ymgyt) ([fa6f178c](https://github.com/ymgyt/syndicationd/commit/fa6f178cbad30c6100cde0a9c77ca2eed1eadb52))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.8...synd-term-v0.1.9


## [synd-term-v0.1.8] - 2024-03-17

### 📡 Features

- Use nerd fond in prompt by [@ymgyt](https://github.com/ymgyt) ([b864e277](https://github.com/ymgyt/syndicationd/commit/b864e27793087d12ea63a5215df509c25854ac46))
- Use nerd font in columns by [@ymgyt](https://github.com/ymgyt) ([278fbbe8](https://github.com/ymgyt/syndicationd/commit/278fbbe833abd770d25c41f7e9e4267514ba2714))
- Support google login by [@ymgyt](https://github.com/ymgyt) ([a55c3109](https://github.com/ymgyt/syndicationd/commit/a55c31094a723e6541300898b8dab875b11a6f4a))
- Error if google jwt email is not verified by [@ymgyt](https://github.com/ymgyt) ([a8ee97d0](https://github.com/ymgyt/syndicationd/commit/a8ee97d05714f9ac9f54df61ddfc8ea1bca2cea8))
- Add nerd font to feed detail component by [@ymgyt](https://github.com/ymgyt) ([1379a297](https://github.com/ymgyt/syndicationd/commit/1379a297d6129675225f16c578aa51930293cda9))

### 🧹 Refactor

- Rename device flow poll method by [@ymgyt](https://github.com/ymgyt) ([19fe8c4f](https://github.com/ymgyt/syndicationd/commit/19fe8c4fbce84cc8dab6678d38653fa304bd26ff))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.7...synd-term-v0.1.8


## [synd-term-v0.1.7] - 2024-03-12

### 📡 Features

- Show first graphql error in ui by [@ymgyt](https://github.com/ymgyt) ([ca29ea02](https://github.com/ymgyt/syndicationd/commit/ca29ea021937f9818555dd64659041da04762f15))
- Add export command by [@ymgyt](https://github.com/ymgyt) ([9bb73182](https://github.com/ymgyt/syndicationd/commit/9bb731820e1f064f1d5776f5285ea57785596006))
- Print export json schema by [@ymgyt](https://github.com/ymgyt) ([3023c3f7](https://github.com/ymgyt/syndicationd/commit/3023c3f74824d4001b7d684b53b3e4e978384302))

### ⚙️ Miscellaneous Tasks

- Configure oranda changelog by [@ymgyt](https://github.com/ymgyt) ([1aecb8ce](https://github.com/ymgyt/syndicationd/commit/1aecb8ce5a31b766d63d8bb283e993508e379608))
- Specify features to build by [@ymgyt](https://github.com/ymgyt) ([b7db28d1](https://github.com/ymgyt/syndicationd/commit/b7db28d1eb796899a48cf23366499e287fe775fa))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.6...synd-term-v0.1.7


## [synd-term-v0.1.6] - 2024-02-28

### 📡 Features

- Add move to first/last commnad by [@ymgyt](https://github.com/ymgyt) ([4bc7f482](https://github.com/ymgyt/syndicationd/commit/4bc7f482d10e52339057784052194d8ddeff30b5))

### ⚙️ Miscellaneous Tasks

- Read changelog by [@ymgyt](https://github.com/ymgyt) ([9095f676](https://github.com/ymgyt/syndicationd/commit/9095f6764cf8ee1bf2acff85f4df4250bb0e4167))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.5...synd-term-v0.1.6


## [synd-term-v0.1.5] - 2024-02-25

### 📡 Features

- Use env var as default flag value by [@ymgyt](https://github.com/ymgyt) ([c7887e92](https://github.com/ymgyt/syndicationd/commit/c7887e925d6856f761051d118662d77d35d08968))
- Handle subscribe feed error by [@ymgyt](https://github.com/ymgyt) ([d6abb26e](https://github.com/ymgyt/syndicationd/commit/d6abb26eb7ea75ba479f07cb83ff680a1708c6af))
- Use entry updated if published is none by [@ymgyt](https://github.com/ymgyt) ([2b16b51c](https://github.com/ymgyt/syndicationd/commit/2b16b51c3cadb7b0dd74a848ae43ff078372b678))
- Add feed detail widget by [@ymgyt](https://github.com/ymgyt) ([836258d4](https://github.com/ymgyt/syndicationd/commit/836258d490bd63de7bc481bc6ad9f5866f5e861e))
- Add reload by [@ymgyt](https://github.com/ymgyt) ([de11397c](https://github.com/ymgyt/syndicationd/commit/de11397cde2d003b81eb029752673f214724c4c2))
- Add client timeout flag by [@ymgyt](https://github.com/ymgyt) ([86f5bf43](https://github.com/ymgyt/syndicationd/commit/86f5bf437193791905d82b441f9bc39cecc401ff))
- Add check command by [@ymgyt](https://github.com/ymgyt) ([018c0c22](https://github.com/ymgyt/syndicationd/commit/018c0c222704746315e3a0faf852a0868f719a00))

### 🐛 Bug Fixes

- Use name instead of bin_name by [@ymgyt](https://github.com/ymgyt) ([1a9b81dd](https://github.com/ymgyt/syndicationd/commit/1a9b81dd6a9734ea99d63bac052b73b55e9470fd))

### ⚙️ Miscellaneous Tasks

- Set clap bin_name by [@ymgyt](https://github.com/ymgyt) ([dca2b898](https://github.com/ymgyt/syndicationd/commit/dca2b898b2cd596b0655797e037c5a5c82cf9b0a))
- Configure feed detail height by [@ymgyt](https://github.com/ymgyt) ([3de1c98f](https://github.com/ymgyt/syndicationd/commit/3de1c98fdce08a622323c269373d2ece0b00ec74))
- Typo by [@ymgyt](https://github.com/ymgyt) ([13ccdb5d](https://github.com/ymgyt/syndicationd/commit/13ccdb5d7c80627913d9858887b7b6d84dc07dff))
- Trim prefix from changelog by [@ymgyt](https://github.com/ymgyt) ([95d44877](https://github.com/ymgyt/syndicationd/commit/95d448773ec7ab009fbece0928854364679b6f2c))
- Set brew fomula name by [@ymgyt](https://github.com/ymgyt) ([8b33da9a](https://github.com/ymgyt/syndicationd/commit/8b33da9afc98ab6cdc12a0ca48829b27f39c63f6))
- Change default endpoint by [@ymgyt](https://github.com/ymgyt) ([c352b871](https://github.com/ymgyt/syndicationd/commit/c352b8713f4acbaf022c857e036d33fc688c9991))
- Add homepage to package metadata by [@ymgyt](https://github.com/ymgyt) ([4bfdb49e](https://github.com/ymgyt/syndicationd/commit/4bfdb49e317e18ff6345ce1b8e8071f0497a1a5f))
- Use workspace dep by [@ymgyt](https://github.com/ymgyt) ([92163422](https://github.com/ymgyt/syndicationd/commit/921634227a53e2a3594d1cedb5116e53dc43baa4))
- Enable cargo-dist explicitly by [@ymgyt](https://github.com/ymgyt) ([3a04e732](https://github.com/ymgyt/syndicationd/commit/3a04e7327a752dea0497f900f0a96364977de96e))

### 📚 Documentation

- Update install description by [@ymgyt](https://github.com/ymgyt) ([13ecd094](https://github.com/ymgyt/syndicationd/commit/13ecd094ae813517d7554c54572dcc2a83654311))
- Configure oranda by [@ymgyt](https://github.com/ymgyt) ([91e158df](https://github.com/ymgyt/syndicationd/commit/91e158df904e91a27d8f68217500ad76ea91ffe9))
- Configure oranda social by [@ymgyt](https://github.com/ymgyt) ([1624d62a](https://github.com/ymgyt/syndicationd/commit/1624d62a51fdeea38594869c707d036c792f2e61))
- Configure oranda components by [@ymgyt](https://github.com/ymgyt) ([3dcbc57a](https://github.com/ymgyt/syndicationd/commit/3dcbc57a435321d7f39e7e39bf90b44b1b712e7b))

### 🔧 Testing

- Use tempfile instead of deprecated tempdir by [@ymgyt](https://github.com/ymgyt) ([749de1db](https://github.com/ymgyt/syndicationd/commit/749de1dba0235e30e1e79ca10849d049005c0a15))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.3...synd-term-v0.1.5


## [synd-term-v0.1.3] - 2024-02-19

### 📡 Features

- Improve feed url parse by [@ymgyt](https://github.com/ymgyt) ([460e87d0](https://github.com/ymgyt/syndicationd/commit/460e87d00f9acd83a922a97c339a620c0037c14f))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.2...synd-term-v0.1.3


## [synd-term-v0.1.2] - 2024-02-19

### 📡 Features

- Change log directive env key by [@ymgyt](https://github.com/ymgyt) ([6da13de1](https://github.com/ymgyt/syndicationd/commit/6da13de165e3ec57e3c15c08dc6f8237debe082e))

### ⚙️ Miscellaneous Tasks

- Use include by [@ymgyt](https://github.com/ymgyt) ([cdff7a76](https://github.com/ymgyt/syndicationd/commit/cdff7a762d6cca85160f01b6f646d8baa6e47e59))

### 🧹 Refactor

- Remove comment by [@ymgyt](https://github.com/ymgyt) ([2b042c69](https://github.com/ymgyt/syndicationd/commit/2b042c696d5c64c3dd2608e4382f50e4a76ed38c))

https://github.com/ymgyt/syndicationd/compare/synd-term-v0.1.1...synd-term-v0.1.2


## [synd-term-v0.1.1] - 2024-02-12

### 📡 Features

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

### 🐛 Bug Fixes

- Workarround scrollbar rendering bug by [@ymgyt](https://github.com/ymgyt) ([d2982cb6](https://github.com/ymgyt/syndicationd/commit/d2982cb6c8fa385655290d953aa9243d3470382d))
- Build by [@ymgyt](https://github.com/ymgyt) ([bd340e9d](https://github.com/ymgyt/syndicationd/commit/bd340e9d30f101c891f53b2d2be10a0cf8833f4b))

### ⚙️ Miscellaneous Tasks

- Format toml by [@ymgyt](https://github.com/ymgyt) ([36677745](https://github.com/ymgyt/syndicationd/commit/3667774506106fe0f38d77efac9f4b27c70090aa))
- Update CHANGELOG by [@ymgyt](https://github.com/ymgyt) ([f5091f3c](https://github.com/ymgyt/syndicationd/commit/f5091f3ceff04b9ff818bb4e0ce0e4bbe9851177))
- Update CHANGELOG by [@ymgyt](https://github.com/ymgyt) ([99f036df](https://github.com/ymgyt/syndicationd/commit/99f036dfe227c1670f967aa949116e3ae8a2c97b))
- Use hyphen as package name instead of underscore by [@ymgyt](https://github.com/ymgyt) ([0a8ed059](https://github.com/ymgyt/syndicationd/commit/0a8ed05997790f9f05c932c92fa2b2b2d74065a9))
- Update CHANGELOG by [@ymgyt](https://github.com/ymgyt) ([e688670c](https://github.com/ymgyt/syndicationd/commit/e688670c853718a1cb825cb787861dffe55046d1))
- Rename synd-authn to synt-auth to publish as a new crate by [@ymgyt](https://github.com/ymgyt) ([59ae4ffa](https://github.com/ymgyt/syndicationd/commit/59ae4ffa51f5323fa4a3aae5e30e950b15730519))
- Update CHANGELOG by [@ymgyt](https://github.com/ymgyt) ([31eb8a34](https://github.com/ymgyt/syndicationd/commit/31eb8a3472e770931fab427e2a8c74a9754b157a))
- Update CHANGELOG by [@ymgyt](https://github.com/ymgyt) ([e1910ce1](https://github.com/ymgyt/syndicationd/commit/e1910ce120ca9d8f38fa4c479156f723d54ae59c))

### 🔧 Testing

- Impl device flow test case by [@ymgyt](https://github.com/ymgyt) ([93572902](https://github.com/ymgyt/syndicationd/commit/9357290265a4fbf8d78721e4f9f1904b1cf5b12a))
- Add auth flow case by [@ymgyt](https://github.com/ymgyt) ([6d2b1905](https://github.com/ymgyt/syndicationd/commit/6d2b1905d9b06bd9ed670f210cd590f89405c37c))
- Run kvsd in test by [@ymgyt](https://github.com/ymgyt) ([923e65a1](https://github.com/ymgyt/syndicationd/commit/923e65a131bed1a0a10d073b0eb9d5091cc184fe))
- Run integration test by [@ymgyt](https://github.com/ymgyt) ([20c0bc2d](https://github.com/ymgyt/syndicationd/commit/20c0bc2d31a938d3103fafedba5a10b4a9bba9ae))
- Fix tls conf path by [@ymgyt](https://github.com/ymgyt) ([e3d764a4](https://github.com/ymgyt/syndicationd/commit/e3d764a453b527a98b1eaf268ead67469c0e192d))

### 🧹 Refactor

- Rename crates by [@ymgyt](https://github.com/ymgyt) ([ce0982e4](https://github.com/ymgyt/syndicationd/commit/ce0982e497647b23dcf07e39d525121bcd9ac1fa))
- Create synd_authn crate by [@ymgyt](https://github.com/ymgyt) ([682bcc6f](https://github.com/ymgyt/syndicationd/commit/682bcc6ff3c035be566dea99d2487e0173537c8d))
- Use clippy pedantic by [@ymgyt](https://github.com/ymgyt) ([328ddade](https://github.com/ymgyt/syndicationd/commit/328ddadebbad5381271c5e84cce2d6888252e70c))
- Clippy by [@ymgyt](https://github.com/ymgyt) ([a1693b36](https://github.com/ymgyt/syndicationd/commit/a1693b36b73ad3987af9a853e214392d8b1eae8d))
- Fix lint by [@ymgyt](https://github.com/ymgyt) ([aac00b98](https://github.com/ymgyt/syndicationd/commit/aac00b98335bb75cc57fdea0875bfd675bf8f3cc))
- Rename tab by [@ymgyt](https://github.com/ymgyt) ([be4add1e](https://github.com/ymgyt/syndicationd/commit/be4add1e261c505d87b174795274236fd8ce46e7))

https://github.com/ymgyt/syndicationd/compare/...synd-term-v0.1.1


<!-- generated by git-cliff -->
