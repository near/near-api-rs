use serde::{Deserialize, Serialize};

use crate::{
    AccessKey, Account, AccountId, CryptoHash, DelayedReceipt, PublicKey, Receipt, StoreKey,
    StoreValue,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateRecord {
    Account {
        account_id: AccountId,
        account: Account,
    },
    Data {
        account_id: AccountId,
        data_key: StoreKey,
        value: StoreValue,
    },
    Contract {
        account_id: AccountId,
        code: Vec<u8>,
    },
    AccessKey {
        account_id: AccountId,
        public_key: PublicKey,
        access_key: AccessKey,
    },
    PostponedReceipt(Box<Receipt>),
    ReceivedData {
        account_id: AccountId,
        data_id: CryptoHash,
        data: Option<Vec<u8>>,
    },
    DelayedReceipt(DelayedReceipt),
}
