//! A Rust library for interacting with the NEAR Protocol blockchain
//!
//! This crate provides a high-level API for interacting with NEAR Protocol, including:
//! - [Account management and creation](Account)
//! - [Contract deployment and interaction with it](Contract)
//! - [Token operations](Tokens) ([`NEAR`](https://docs.near.org/concepts/basics/tokens), [`FT`](https://docs.near.org/build/primitives/ft), [`NFT`](https://docs.near.org/build/primitives/nft))
//! - [Storage management](StorageDeposit)
//! - [Staking operations](Staking)
//! - [Custom transaction building and signing](Transaction)
//! - [Querying the chain data](Chain)
//! - [Several ways to sign the transaction](signer)
//! - Account nonce caching and access-key pooling mechanisms to speed up the transaction processing.
//! - Support for backup RPC endpoints
//!
//! # Example
//! In this example, we use Bob account with a predefined seed phrase to create Alice account and pre-fund it with 1 `NEAR`.
//! ```rust,no_run
//! use near_api::{*, signer::generate_secret_key};
//! use std::str::FromStr;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize network configuration
//! let bob = AccountId::from_str("bob.testnet")?;
//! let bob_seed_phrase = "lucky barrel fall come bottom can rib join rough around subway cloth ";
//!
//! // Fetch NEAR balance
//! let _bob_balance = Tokens::account(bob.clone())
//!     .near_balance()
//!     .fetch_from_testnet()
//!     .await?;
//!
//! // Create an account instance
//! let signer = Signer::new(Signer::from_seed_phrase(bob_seed_phrase, None)?)?;
//! let alice_secret_key = generate_secret_key()?;
//! Account::create_account(AccountId::from_str("alice.testnet")?)
//!     .fund_myself(bob.clone(), NearToken::from_near(1))
//!     .with_public_key(alice_secret_key.public_key())
//!     .with_signer(signer)
//!     .send_to_testnet()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//! - `ledger`: Enables hardware wallet support
//! - `keystore`: Enables system keychain integration
//! - `workspaces`: Enables integration with near-workspaces for testing

mod account;
mod chain;
mod config;
mod contract;
mod stake;
mod storage;
mod tokens;
mod transactions;

// TODO: to be honest, there is almost nothing in this file
// we should maybe integrate with them more tightly
// for now, i comment it out
// mod fastnear;

mod common;

pub use near_api_types as types;
pub mod errors;
pub mod signer;

pub use crate::{
    account::Account,
    chain::Chain,
    config::{NetworkConfig, RPCEndpoint, RetryMethod},
    contract::Contract,
    signer::{Signer, SignerTrait},
    stake::{Delegation, Staking},
    storage::{StorageDeposit, StorageDepositBuilder, StorageUnregisterBuilder},
    tokens::Tokens,
    transactions::Transaction,
    types::{
        tokens::{FTBalance, USDT_BALANCE, W_NEAR_BALANCE},
        AccountId, CryptoHash, Data, EpochReference, NearGas, NearToken, PublicKey, Reference,
        SecretKey,
    },
};

pub mod advanced {
    pub use crate::common::query::*;
    pub use crate::common::send::*;
}
