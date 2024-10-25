# near-api
The near-api is a simple Rust library that helps developers interact easily with the NEAR blockchain. The library was highly inspired by the API of the [near-cli-rs](https://github.com/near/near-cli-rs) library. The library extensively utilizes builder patterns, this way we guide the users through the user flow, preventing most of the errors and focusing on each step.

Currently, the library provides:
* Account management
* Contract deployment and interaction
* NEAR, FT, NFT transfers
* Storage deposit management
* Stake management
* Ability to create custom transactions
* Several ways to sign transactions (SecretKey, Seedphrase, File, Ledger, Secure keychain).
* Account key pool support to sign transaction with different user keys to avoid nonce issues.

## Current issues

The library is already usable and might be used for rapid prototyping, it lacks some points to make it production-ready:
- [ ] documentation + examples
- [ ] integration tests for all API calls
- [x] CI
- [x] anyhow -> thiserror
- [x] ledger is blocking and it's not good in the async runtime
- [ ] secure keychain is not that straightforward to use
- [x] storage deposit manager for FT calls 
- [x] basic logging with tracing for querying/signing/sending transactions

## Examples
The crate provides [examples](./examples/) that contain detailed information on using the library.
 
