use near_primitives::types::BlockHeight;

pub mod account;
pub mod config;
pub mod contract;
pub mod fastnear;
pub mod query;
pub mod stake;
pub mod types;

pub mod send;
pub mod sign;
pub mod signed_delegate_action;
pub mod transactions;

const META_TRANSACTION_VALID_FOR_DEFAULT: BlockHeight = 1000;
