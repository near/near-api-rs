# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
