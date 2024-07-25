use std::marker::PhantomData;

use near_contract_standards::{
    fungible_token::metadata::FungibleTokenMetadata,
    non_fungible_token::{metadata::NFTContractMetadata, Token},
};
use near_primitives::{
    action::{Action, TransferAction},
    types::{AccountId, BlockReference},
    views::AccountView,
};
use near_sdk::json_types::U128;
use near_token::NearToken;
use serde_json::json;

use crate::{
    common::{
        query::{
            AccountViewHandler, CallResultHandler, MultiQueryBuilder, MultiQueryHandler,
            PostprocessHandler, QueryBuilder, SimpleQuery,
        },
        send::Transactionable,
    },
    contract::Contract,
    transactions::{ConstructTransaction, TransactionWithSign},
    types::{
        tokens::{FTBalance, UserBalance},
        transactions::PrepopulateTransaction,
        Data,
    },
};

#[derive(Debug, Clone)]
pub struct Tokens {
    account_id: AccountId,
}

impl Tokens {
    pub fn of(account_id: AccountId) -> Self {
        Self { account_id }
    }

    pub fn near_balance(self) -> QueryBuilder<PostprocessHandler<UserBalance, AccountViewHandler>> {
        let request = near_primitives::views::QueryRequest::ViewAccount {
            account_id: self.account_id,
        };

        QueryBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            PostprocessHandler::new(
                AccountViewHandler,
                Box::new(|account: Data<AccountView>| {
                    let account = account.data;
                    let liquid = NearToken::from_yoctonear(account.amount);
                    let locked = NearToken::from_yoctonear(account.locked);
                    let storage_usage = account.storage_usage;
                    UserBalance {
                        liquid,
                        locked,
                        storage_usage,
                    }
                }),
            ),
        )
    }

    pub fn nft_metadata(
        contract_id: AccountId,
    ) -> anyhow::Result<QueryBuilder<CallResultHandler<NFTContractMetadata>>> {
        Ok(Contract(contract_id)
            .call_function("nft_metadata", ())?
            .read_only())
    }

    pub fn nft_assets(
        self,
        nft_contract: AccountId,
    ) -> anyhow::Result<QueryBuilder<CallResultHandler<Vec<Token>>>> {
        Ok(Contract(nft_contract)
            .call_function(
                "nft_tokens_for_owner",
                json!({
                    "account_id": self.account_id.to_string(),
                }),
            )?
            .read_only())
    }

    pub fn ft_metadata(
        contract_id: AccountId,
    ) -> anyhow::Result<QueryBuilder<CallResultHandler<FungibleTokenMetadata>>> {
        Ok(Contract(contract_id)
            .call_function("ft_metadata", ())?
            .read_only())
    }

    pub fn ft_balance(
        self,
        ft_contract: AccountId,
    ) -> anyhow::Result<
        MultiQueryBuilder<
            PostprocessHandler<
                FTBalance,
                MultiQueryHandler<(
                    CallResultHandler<FungibleTokenMetadata>,
                    CallResultHandler<U128>,
                )>,
            >,
        >,
    > {
        let postprocess = PostprocessHandler::new(
            MultiQueryHandler::new((
                CallResultHandler(PhantomData::<FungibleTokenMetadata>),
                CallResultHandler(PhantomData::<U128>),
            )),
            |(metadata, amount)| {
                FTBalance::with_decimals(metadata.data.decimals).with_amount(amount.data.0)
            },
        );

        let query_builder = MultiQueryBuilder::new(postprocess, BlockReference::latest())
            .add_query_builder(Self::ft_metadata(ft_contract.clone())?)
            .add_query_builder(
                Contract(ft_contract)
                    .call_function(
                        "ft_balance_of",
                        json!({
                            "account_id": self.account_id
                        }),
                    )?
                    .read_only::<()>(),
            );

        Ok(query_builder)
    }

    pub fn send_to(self, receiver_id: AccountId) -> SendTo {
        SendTo {
            from: self.account_id,
            receiver_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SendTo {
    from: AccountId,
    receiver_id: AccountId,
}

impl SendTo {
    pub fn near(self, amount: NearToken) -> ConstructTransaction {
        ConstructTransaction::new(self.from, self.receiver_id).add_action(Action::Transfer(
            TransferAction {
                deposit: amount.as_yoctonear(),
            },
        ))
    }

    pub fn ft(
        self,
        ft_contract: AccountId,
        amount: FTBalance,
    ) -> anyhow::Result<TransactionWithSign<FTTransactionable>> {
        let tr = Contract(ft_contract)
            .call_function(
                "ft_transfer",
                json!({
                    "receiver_id": self.receiver_id,
                    "amount": U128(amount.amount()),
                }),
            )?
            .transaction()
            .deposit(NearToken::from_yoctonear(1))
            .with_signer_account(self.from);

        Ok(TransactionWithSign {
            tx: FTTransactionable {
                prepopulated: tr.tr,
                decimals: amount.decimals(),
            },
        })
    }

    pub fn nft(
        self,
        nft_contract: AccountId,
        token_id: String,
    ) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(nft_contract)
            .call_function(
                "nft_transfer",
                json!({
                    "receiver_id": self.receiver_id,
                    "token_id": token_id
                }),
            )?
            .transaction()
            .with_signer_account(self.from))
    }
}

pub struct FTTransactionable {
    prepopulated: PrepopulateTransaction,
    decimals: u8,
}

impl Transactionable for FTTransactionable {
    type Handler = CallResultHandler<FungibleTokenMetadata>;

    fn prepopulated(&self) -> PrepopulateTransaction {
        self.prepopulated.clone()
    }

    fn validate_with_network(
        &self,
        _network: &crate::NetworkConfig,
        query_response: Option<Data<FungibleTokenMetadata>>,
    ) -> anyhow::Result<()> {
        let metadata = query_response.ok_or_else(|| anyhow::anyhow!("No metadata found"))?;
        if metadata.data.decimals != self.decimals {
            return Err(anyhow::anyhow!(
                "Decimals mismatch: expected {}, got {}",
                metadata.data.decimals,
                self.decimals,
            ));
        }
        Ok(())
    }

    fn prequery(&self) -> Option<QueryBuilder<Self::Handler>> {
        Tokens::ft_metadata(self.prepopulated.receiver_id.clone()).ok()
    }
}
