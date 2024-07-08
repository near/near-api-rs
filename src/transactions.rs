use near_primitives::{
    action::Action,
    hash::CryptoHash,
    types::{AccountId, BlockHeight, Nonce},
};

use crate::sign::SignSeedPhrase;

#[derive(Debug, Clone)]
pub struct PrepopulateTransaction {
    pub signer_id: AccountId,
    pub receiver_id: AccountId,
    pub actions: Vec<Action>,

    pub nonce: Option<u64>,
    pub block_hash: Option<CryptoHash>,
    pub block_height: Option<u64>,

    pub meta_transaction_valid_for: Option<BlockHeight>,
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
                nonce: None,
                block_hash: None,
                block_height: None,
                meta_transaction_valid_for: None,
            },
        }
    }

    pub fn add_action(mut self, action: Action) -> Self {
        self.tr.actions.push(action);
        self
    }

    pub fn block_hash(mut self, hash: CryptoHash) -> Self {
        self.tr.block_hash = Some(hash);
        self
    }

    pub fn block_height(mut self, height: BlockHeight) -> Self {
        self.tr.block_height = Some(height);
        self
    }

    pub fn nonce(mut self, nonce: Nonce) -> Self {
        self.tr.nonce = Some(nonce);
        self
    }

    pub fn with_seed(self, phrase: String) -> SignSeedPhrase {
        SignSeedPhrase::new(phrase, self.tr)
    }

    pub fn meta_transaction_valid_for(mut self, blocks: BlockHeight) -> Self {
        self.tr.meta_transaction_valid_for = Some(blocks);

        self
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

    #[tokio::test]
    async fn send_transfer() {
        let signer_id: AccountId = "yurtur.testnet".to_string().parse().unwrap();
        let receiver_id: AccountId = "race-of-sloths.testnet".to_string().parse().unwrap();

        super::Transaction::construct(signer_id.clone(), receiver_id.clone())
            .add_action(Action::Transfer(TransferAction {
                deposit: NearToken::from_millinear(100u128).as_yoctonear(),
            }))
            .with_seed(include_str!("../seed_phrase").to_string())
            .sign_for_testnet()
            .await
            .unwrap()
            .send_to_testnet()
            .await
            .unwrap()
            .assert_success();

        super::Transaction::construct(signer_id, receiver_id)
            .add_action(Action::Transfer(TransferAction {
                deposit: NearToken::from_millinear(100u128).as_yoctonear(),
            }))
            .with_seed(include_str!("../seed_phrase").to_string())
            .sign_meta_for_testnet()
            .await
            .unwrap()
            .send_to_testnet()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }
}
