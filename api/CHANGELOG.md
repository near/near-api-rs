# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
