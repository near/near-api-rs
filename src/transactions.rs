use near_primitives::{action::Action, types::AccountId};

use crate::{
    send::{ExecuteMetaTransaction, ExecuteSignedTransaction},
    sign::Signer,
};

#[derive(Debug, Clone)]
pub struct PrepopulateTransaction {
    pub signer_id: AccountId,
    pub receiver_id: AccountId,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone)]

pub struct ConstructTransaction {
    pub tr: PrepopulateTransaction,
}

impl ConstructTransaction {
    pub fn new(signer_id: AccountId, receiver_id: AccountId) -> Self {
        Self {
            tr: PrepopulateTransaction {
                signer_id,
                receiver_id,
                actions: Vec::new(),
            },
        }
    }

    pub fn add_action(mut self, action: Action) -> Self {
        self.tr.actions.push(action);
        self
    }

    pub fn signer(self, signer: Signer) -> ExecuteSignedTransaction {
        ExecuteSignedTransaction::new(self.tr, signer.into())
    }

    pub fn meta_signer(self, signer: Signer) -> ExecuteMetaTransaction {
        ExecuteMetaTransaction::new(self.tr, signer.into())
    }
}

pub struct Transaction;

impl Transaction {
    pub fn construct(signer_id: AccountId, receiver_id: AccountId) -> ConstructTransaction {
        ConstructTransaction::new(signer_id, receiver_id)
    }
}

#[cfg(test)]
mod tests {
    use near_primitives::{
        action::{Action, TransferAction},
        types::AccountId,
    };
    use near_token::NearToken;

    use crate::{sign::Signer, transactions::Transaction};

    #[tokio::test]
    async fn send_transfer() {
        let signer_id: AccountId = "yurtur.testnet".to_string().parse().unwrap();
        let receiver_id: AccountId = "race-of-sloths.testnet".to_string().parse().unwrap();

        Transaction::construct(signer_id.clone(), receiver_id.clone())
            .add_action(Action::Transfer(TransferAction {
                deposit: NearToken::from_millinear(100u128).as_yoctonear(),
            }))
            .signer(Signer::seed_phrase(
                include_str!("../seed_phrase").to_string(),
            ))
            .presign_with_mainnet()
            .send_to_testnet()
            .await
            .unwrap()
            .assert_success();

        Transaction::construct(signer_id, receiver_id)
            .add_action(Action::Transfer(TransferAction {
                deposit: NearToken::from_millinear(100u128).as_yoctonear(),
            }))
            .meta_signer(Signer::seed_phrase(
                include_str!("../seed_phrase").to_string(),
            ))
            .presign_offline(block_hash, nonce, block_height)
            .unwrap()
            .send_to_testnet()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }
}
