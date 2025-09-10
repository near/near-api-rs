# near-api
<p>
    <a href="https://docs.rs/near-api"><img src="https://docs.rs/near-api/badge.svg?style=flat-square" alt="Reference Documentation" /></a>
    <a href="https://crates.io/crates/near-api"><img src="https://img.shields.io/crates/v/near-api.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/near-api"><img src="https://img.shields.io/crates/d/near-api.svg?style=flat-square" alt="Download" /></a>
    <a href="https://near.chat"><img src="https://img.shields.io/discord/490367152054992913?style=flat-square&label=discord&color=lightgreen" alt="Join the community on Discord" /></a>
    <a href="https://t.me/NEAR_Tools_Community_Group"><img src="https://img.shields.io/badge/telegram-online-lightgreen?style=flat-square" alt="Join the community on Telegram" /></a>
 </p>

The `near-api` is a simple Rust library that helps developers interact easily with the NEAR blockchain. The library was highly inspired by the API of the [`near-cli-rs`](https://github.com/near/near-cli-rs) library. The library extensively utilizes builder patterns, this way we guide the users through the user flow, preventing most of the errors and focusing on each step.

Currently, the library provides:
* Account management
* Contract deployment and interaction
* NEAR, FT, NFT transfers
* Storage deposit management
* Stake management
* Ability to create custom transactions
* Several ways to sign transactions (secret key, seed phrase, file, ledger, secure keychain).
* Account key pool support to sign the transaction with different user keys to avoid nonce issues.

The minimum required version is located in the [rust-version](./Cargo.toml#L4) field of the `Cargo.toml` file.

## Features

* `ledger`: Enables integration with a Ledger hardware signer for secure key management.
* `keystore`: Enables integration with a system keystore signer for managing keys securely on the local system.
* `workspaces`: Provides integration with [`near-workspaces`](https://github.com/near/near-workspaces-rs) for testing purposes. This feature allows you to convert `near-workspaces` networks (such as sandbox, testnet, etc.) into a NetworkConfig and use `near-workspaces` `Account` object as a signer for testing and development.

## Current issues

The library is in good condition, but lacks a few points to be even better:
- [x] documentation
- [ ] good quality examples
- [ ] integration tests for all API calls
- [x] CI
- [x] anyhow -> thiserror
- [x] ledger is blocking and it's not good in the async runtime
- [ ] secure keychain is not that straightforward to use
- [x] storage deposit manager for FT calls
- [x] basic logging with tracing for querying/signing/sending transactions
- [x] self-sustainable. remove the `nearcore` as a dependency ([#5](https://github.com/near/near-api-rs/issues/5))

## Examples
The crate provides [examples](./examples/) that contain detailed information on using the library.
