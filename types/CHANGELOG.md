# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.0](https://github.com/near/near-api-rs/compare/near-api-types-v0.7.8...near-api-types-v0.8.0) - 2025-12-04

### Added

- Add NEP-413 message verification ([#98](https://github.com/near/near-api-rs/pull/98))
- updated acount id to v2, updated openapi-types, fixed DeterministicStateInit action serialization ([#97](https://github.com/near/near-api-rs/pull/97))

### Other

- restricted usage of unwrap and expects, removed some unwraps in signing ([#96](https://github.com/near/near-api-rs/pull/96))
- [**breaking**] defer errors for contract interaction ([#93](https://github.com/near/near-api-rs/pull/93))

## [0.7.8](https://github.com/near/near-api-rs/compare/near-api-types-v0.7.7...near-api-types-v0.7.8) - 2025-11-26

Synchronize version with near-api


## [0.7.3](https://github.com/near/near-api-rs/compare/near-api-types-v0.7.2...near-api-types-v0.7.3) - 2025-11-10

### Added

- added assert_failure method similar to assert_success
- helper methods to improve dev and test experience ([#83](https://github.com/near/near-api-rs/pull/83))

## [0.7.2](https://github.com/near/near-api-rs/compare/near-api-types-v0.7.1...near-api-types-v0.7.2) - 2025-11-03

### Other

- restored back to 2021 edition

## [0.7.1](https://github.com/near/near-api-rs/compare/near-api-types-v0.7.0...near-api-types-v0.7.1) - 2025-10-28

### Fixed

- *(types)* do not encode in base64 twice ([#71](https://github.com/near/near-api-rs/pull/71))

## [0.7.0](https://github.com/near/near-api-rs/compare/near-api-types-v0.6.1...near-api-types-v0.7.0) - 2025-10-13

### Added

- removed near-sandbox from near-api ([#69](https://github.com/near/near-api-rs/pull/69))
- updated types to the latest version, added support for DeterministicAccountStateInit ([#68](https://github.com/near/near-api-rs/pull/68))
