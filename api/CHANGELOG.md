# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.0](https://github.com/near/near-api-rs/compare/near-api-v0.7.8...near-api-v0.8.0) - 2025-12-04

### Added

- Add NEP-413 message verification ([#98](https://github.com/near/near-api-rs/pull/98))
- [**breaking**] signer interface improvement ([#89](https://github.com/near/near-api-rs/pull/89))

### Other

- clean up comments in utils.rs
- [**breaking**] simplify and remove PublicKeyProvider to improve code clarity ([#95](https://github.com/near/near-api-rs/pull/95))
- restricted usage of unwrap and expects, removed some unwraps in signing ([#96](https://github.com/near/near-api-rs/pull/96))
- migrated examples and tests to TestResult from unwraps
- [**breaking**] defer errors for contract interaction ([#93](https://github.com/near/near-api-rs/pull/93))

## [0.7.7](https://github.com/near/near-api-rs/compare/near-api-v0.7.6...near-api-v0.7.7) - 2025-11-10

### Added

- helper methods to improve dev and test experience ([#83](https://github.com/near/near-api-rs/pull/83))
- added `global_wasm` method to Contract struct ([#81](https://github.com/near/near-api-rs/pull/81))

## [0.7.6](https://github.com/near/near-api-rs/compare/near-api-v0.7.5...near-api-v0.7.6) - 2025-11-03

### Other

- fixed MultiQueryResponseHandler for 3 queries

## [0.7.5](https://github.com/near/near-api-rs/compare/near-api-v0.7.4...near-api-v0.7.5) - 2025-11-03

### Other

- restored back to 2021 edition

## [0.7.4](https://github.com/near/near-api-rs/compare/near-api-v0.7.3...near-api-v0.7.4) - 2025-10-31

### Added

- added ft_transfer_call and nft_transfer_call support

### Other

- deprecate error invariant
- do not fail if NoMetadata in token
- use NearToken for serialization

## [0.7.3](https://github.com/near/near-api-rs/compare/near-api-v0.7.2...near-api-v0.7.3) - 2025-10-30

### Added

- added `max_gas` to transaction construction as well

## [0.7.2](https://github.com/near/near-api-rs/compare/near-api-v0.7.1...near-api-v0.7.2) - 2025-10-30

### Added

- added `call_function_raw` and `call_function_borsh` and `max_gas` functions ([#73](https://github.com/near/near-api-rs/pull/73))

### Other

- keep it api compatible

## [0.7.1](https://github.com/near/near-api-rs/compare/near-api-v0.7.0...near-api-v0.7.1) - 2025-10-28

### Other

- updated the following local packages: near-api-types

## [0.7.0](https://github.com/near/near-api-rs/compare/near-api-v0.6.1...near-api-v0.7.0) - 2025-10-13

### Added

- removed near-sandbox from near-api ([#69](https://github.com/near/near-api-rs/pull/69))
- updated types to the latest version, added support for DeterministicAccountStateInit ([#68](https://github.com/near/near-api-rs/pull/68))
- Add Borsh deserialization support for contract calls ([#66](https://github.com/near/near-api-rs/pull/66))
- openapi types ([#64](https://github.com/near/near-api-rs/pull/64))
