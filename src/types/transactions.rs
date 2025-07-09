use omni_transaction::near::types::Action;
use serde::{Deserialize, Serialize};

/// An internal type that represents unsigned transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrepopulateTransaction {
    /// The account that will sign the transaction.
    pub signer_id: near_account_id::AccountId,
    /// The account that will receive the transaction
    pub receiver_id: near_account_id::AccountId,
    /// The actions that will be executed by the transaction.
    pub actions: Vec<Action>,
}
