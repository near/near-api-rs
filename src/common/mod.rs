use near_primitives::types::BlockHeight;

const META_TRANSACTION_VALID_FOR_DEFAULT: BlockHeight = 1000;

pub mod query;
pub mod secret;
pub mod send;
pub mod signed_delegate_action;
