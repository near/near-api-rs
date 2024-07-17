use near_primitives::{action::Action, types::AccountId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrepopulateTransaction {
    pub signer_id: AccountId,
    pub receiver_id: AccountId,
    pub actions: Vec<Action>,
}
