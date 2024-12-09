mod account;
mod chain;
mod config;
mod contract;
mod stake;
mod storage;
mod tokens;
mod transactions;
// TODO: to be honest, there is almost nothing in this file
// we should maybe intergrate with them more tightly
// for now, i comment it out
// mod fastnear;

mod common;

pub mod errors;
pub mod signer;
pub mod types;

pub use crate::{
    account::Account,
    chain::Chain,
    config::{NetworkConfig, RPCEndpoint},
    contract::Contract,
    signer::{Signer, SignerTrait},
    stake::Staking,
    storage::StorageDeposit,
    tokens::Tokens,
    transactions::Transaction,
    types::{
        reference::{EpochReference, Reference},
        tokens::{FTBalance, USDT_BALANCE, W_NEAR_BALANCE},
        Data,
    },
};

pub use near_account_id::AccountId;
pub use near_token::NearToken;
