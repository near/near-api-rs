use std::marker::PhantomData;

use near_contract_standards::{
    fungible_token::metadata::FungibleTokenMetadata,
    non_fungible_token::{metadata::NFTContractMetadata, Token},
};
use near_jsonrpc_client::methods::query::RpcQueryResponse;
use near_primitives::{
    action::{Action, TransferAction},
    types::{AccountId, BlockReference},
    views::AccountView,
};
use near_token::NearToken;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    common::query::{
        AccountViewHandler, CallResultHandler, Data, MultiQueryBuilder, MultiQueryHandler,
        PostprocessHandler, QueryBuilder, SimpleQuery,
    },
    contract::Contract,
    transactions::ConstructTransaction,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FungibleToken {
    pub balance: u128,
    pub decimals: u8,
    pub symbol: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Balance {
    pub liquid: NearToken,
    pub locked: NearToken,
    pub storage_usage: u64,
}

pub struct Tokens(AccountId);

impl Tokens {
    pub fn near_balance(
        self,
    ) -> QueryBuilder<PostprocessHandler<Balance, RpcQueryResponse, AccountViewHandler>> {
        let request = near_primitives::views::QueryRequest::ViewAccount { account_id: self.0 };

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
                    Balance {
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
            .as_read_only())
    }

    pub fn nft_assets(
        self,
        nft_contract: AccountId,
    ) -> anyhow::Result<QueryBuilder<CallResultHandler<Vec<Token>>>> {
        Ok(Contract(nft_contract)
            .call_function(
                "nft_tokens_for_owner",
                json!({
                    "account_id": self.0.to_string(),
                }),
            )?
            .as_read_only())
    }

    pub fn ft_metadata(
        contract_id: AccountId,
    ) -> anyhow::Result<QueryBuilder<CallResultHandler<Vec<FungibleTokenMetadata>>>> {
        Ok(Contract(contract_id)
            .call_function("ft_metadata", ())?
            .as_read_only())
    }

    pub fn ft_balance(
        self,
        ft_contract: AccountId,
    ) -> anyhow::Result<
        MultiQueryBuilder<
            PostprocessHandler<
                FungibleToken,
                RpcQueryResponse,
                MultiQueryHandler<(
                    CallResultHandler<FungibleTokenMetadata>,
                    CallResultHandler<u128>,
                )>,
            >,
        >,
    > {
        let postprocess = PostprocessHandler::new(
            MultiQueryHandler::new((
                CallResultHandler(PhantomData::<FungibleTokenMetadata>),
                CallResultHandler(PhantomData),
            )),
            |(metadata, amount)| FungibleToken {
                balance: amount.data,
                decimals: metadata.data.decimals,
                symbol: metadata.data.symbol,
            },
        );

        let query_builder = MultiQueryBuilder::new(postprocess, BlockReference::latest())
            .add_query_builder(Self::ft_metadata(ft_contract.clone())?)
            .add_query_builder(
                Contract(ft_contract)
                    .call_function(
                        "ft_balance_of",
                        json!({
                            "account_id": self.0
                        }),
                    )?
                    .as_read_only::<()>(),
            );

        Ok(query_builder)
    }

    pub fn send_near(self, receiver_id: AccountId, amount: NearToken) -> ConstructTransaction {
        ConstructTransaction::new(self.0, receiver_id).add_action(Action::Transfer(
            TransferAction {
                deposit: amount.as_yoctonear(),
            },
        ))
    }

    pub fn send_ft(
        self,
        ft_contract: AccountId,
        amount: u128,
    ) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(ft_contract)
            .call_function(
                "ft_transfer",
                json!({
                    "receiver_id": self.0.to_string(),
                    "amount": amount
                }),
            )?
            .as_transaction()
            .with_signer_account(self.0))
    }

    pub fn send_nft(
        self,
        nft_contract: AccountId,
        receiver_id: AccountId,
        token_id: String,
    ) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(nft_contract)
            .call_function(
                "nft_transfer",
                json!({
                    "receiver_id": receiver_id.to_string(),
                    "token_id": token_id
                }),
            )?
            .as_transaction()
            .with_signer_account(self.0))
    }
}
