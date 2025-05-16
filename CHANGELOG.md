# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0](https://github.com/near/near-api-rs/compare/v0.5.0...v0.6.0) - 2025-05-16

### Added

- [**breaking**] added support for the s Global Contracts (NEP-591) ([#56](https://github.com/near/near-api-rs/pull/56))
- [**breaking**] add field output_wasm_path to ContractSourceMetadata ([#55](https://github.com/near/near-api-rs/pull/55))
- add issues & prs to devtools project ([#52](https://github.com/near/near-api-rs/pull/52))

### Fixed

- allow forks to leverage transfer-to-project workflow ([#54](https://github.com/near/near-api-rs/pull/54))

### Other

- [**breaking**] updates near-* dependencies to 0.30 release ([#59](https://github.com/near/near-api-rs/pull/59))
- *(near-contract-standards)* deserialize ContractSourceMetadata::standards field so, as if it were optional 

## [0.5.0](https://github.com/near/near-api-rs/compare/v0.4.0...v0.5.0) - 2025-03-16

### Added

- added `map` method to query builders ([#45](https://github.com/near/near-api-rs/pull/45))
- *(types::contract)* add `BuildInfo` field to `ContractSourceMetadata` ([#46](https://github.com/near/near-api-rs/pull/46))
- [**breaking**] NEP-413 support ([#37](https://github.com/near/near-api-rs/pull/37))

### Other

- [**breaking**] updates near-* dependencies to 0.29 release ([#51](https://github.com/near/near-api-rs/pull/51))
- added rust backward compatibility job, updated project readme ([#48](https://github.com/near/near-api-rs/pull/48))
- [**breaking**] documented types ([#44](https://github.com/near/near-api-rs/pull/44))
- added cargo words to supported dictionary ([#43](https://github.com/near/near-api-rs/pull/43))
- [**breaking**] added spellcheck ([#42](https://github.com/near/near-api-rs/pull/42))
- [**breaking**] documented all the builders. API changes ([#39](https://github.com/near/near-api-rs/pull/39))
- documented network config  ([#35](https://github.com/near/near-api-rs/pull/35))

## [0.4.0](https://github.com/near/near-api-rs/compare/v0.3.0...v0.4.0) - 2024-12-19

### Added

- added ability to specify backup rpc for connecting to the network (#28)
- don't retry on critical errors (query, tx) (#27)

### Other

- updates near-* dependencies to 0.28 release. Removed Cargo.lock (#33)
- [**breaking**] added documentation for root level and signer module (#32)
- added CODEOWNERS (#31)
- removed prelude and filtered entries.  (#29)
- replaced SecretBuilder with utility functions (#26)
- [**breaking**] replaced deploy method as a static method (#18)

## [0.3.0](https://github.com/near/near-api-rs/compare/v0.2.1...v0.3.0) - 2024-11-19

### Added
- added querying block, block hash, and block number ([#9](https://github.com/near/near-api-rs/pull/9))
- added prelude module ([#9](https://github.com/near/near-api-rs/pull/9))

### Other
- [**breaking**] updated near-* dependencies to 0.27 release ([#13](https://github.com/near/near-api-rs/pull/13))

## [0.2.1](https://github.com/near/near-api-rs/compare/v0.2.0...v0.2.1) - 2024-10-25

### Added

- added retry to querying. Simplified retry logic.  ([#7](https://github.com/near/near-api-rs/pull/7))

### Other

- Update README.md
