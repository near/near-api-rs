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
    errors::{BuilderError, FTValidatorError, ValidationError},
    transactions::{ConstructTransaction, TransactionWithSign},
    types::{
        tokens::{FTBalance, UserBalance},
        transactions::PrepopulateTransaction,
        Data,
    },
    NetworkConfig, StorageDeposit,
};

type Result<T> = core::result::Result<T, BuilderError>;

#[derive(Debug, Clone)]
pub struct Tokens {
    account_id: AccountId,
}

impl Tokens {
    pub const fn account(account_id: AccountId) -> Self {
        Self { account_id }
    }

    pub fn near_balance(
        &self,
    ) -> QueryBuilder<PostprocessHandler<UserBalance, AccountViewHandler>> {
        let request = near_primitives::views::QueryRequest::ViewAccount {
            account_id: self.account_id.clone(),
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
    ) -> Result<QueryBuilder<CallResultHandler<NFTContractMetadata>>> {
        Ok(Contract(contract_id)
            .call_function("nft_metadata", ())?
            .read_only())
    }

    pub fn nft_assets(
        &self,
        nft_contract: AccountId,
    ) -> Result<QueryBuilder<CallResultHandler<Vec<Token>>>> {
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
    ) -> Result<QueryBuilder<CallResultHandler<FungibleTokenMetadata>>> {
        Ok(Contract(contract_id)
            .call_function("ft_metadata", ())?
            .read_only())
    }

    #[allow(clippy::complexity)]
    pub fn ft_balance(
        &self,
        ft_contract: AccountId,
    ) -> Result<
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
                            "account_id": self.account_id.clone()
                        }),
                    )?
                    .read_only::<()>(),
            );

        Ok(query_builder)
    }

    pub fn send_to(&self, receiver_id: AccountId) -> SendTo {
        SendTo {
            from: self.account_id.clone(),
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
    ) -> Result<TransactionWithSign<FTTransactionable>> {
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
                receiver: self.receiver_id,
                prepopulated: tr.tr,
                decimals: amount.decimals(),
            },
        })
    }

    pub fn nft(self, nft_contract: AccountId, token_id: String) -> Result<ConstructTransaction> {
        Ok(Contract(nft_contract)
            .call_function(
                "nft_transfer",
                json!({
                    "receiver_id": self.receiver_id,
                    "token_id": token_id
                }),
            )?
            .transaction()
            .deposit(NearToken::from_yoctonear(1))
            .with_signer_account(self.from))
    }
}

#[derive(Clone, Debug)]
pub struct FTTransactionable {
    prepopulated: PrepopulateTransaction,
    receiver: AccountId,
    decimals: u8,
}

impl FTTransactionable {
    pub async fn check_decimals(
        &self,
        network: &NetworkConfig,
    ) -> core::result::Result<(), ValidationError> {
        let metadata = Tokens::ft_metadata(self.prepopulated.receiver_id.clone())?;

        let metadata = metadata
            .fetch_from(network)
            .await
            .map_err(|_| FTValidatorError::NoMetadata)?;
        if metadata.data.decimals != self.decimals {
            Err(FTValidatorError::DecimalsMismatch {
                expected: metadata.data.decimals,
                got: self.decimals,
            })?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Transactionable for FTTransactionable {
    fn prepopulated(&self) -> PrepopulateTransaction {
        self.prepopulated.clone()
    }

    async fn validate_with_network(
        &self,
        network: &NetworkConfig,
    ) -> core::result::Result<(), ValidationError> {
        self.check_decimals(network).await?;

        let storage_balance = StorageDeposit::on_contract(self.prepopulated.receiver_id.clone())
            .view_account_storage(self.receiver.clone())?
            .fetch_from(network)
            .await?;

        if storage_balance.data.is_none() {
            Err(FTValidatorError::StorageDepositNeeded)?;
        }

        Ok(())
    }

    async fn edit_with_network(
        &mut self,
        network: &NetworkConfig,
    ) -> core::result::Result<(), ValidationError> {
        self.check_decimals(network).await?;

        let storage_balance = StorageDeposit::on_contract(self.prepopulated.receiver_id.clone())
            .view_account_storage(self.receiver.clone())?
            .fetch_from(network)
            .await?;

        if storage_balance.data.is_none() {
            let mut action = StorageDeposit::on_contract(self.prepopulated.receiver_id.clone())
                .deposit(self.receiver.clone(), NearToken::from_millinear(100))?
                .with_signer_account(self.prepopulated.signer_id.clone())
                .tr
                .actions;
            action.append(&mut self.prepopulated.actions);
            self.prepopulated.actions = action;
        }
        Ok(())
    }
}
