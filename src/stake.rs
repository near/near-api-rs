use std::collections::BTreeMap;

use near_gas::NearGas;
use near_jsonrpc_client::methods::query::RpcQueryRequest;
use near_primitives::types::{AccountId, BlockReference, EpochReference};
use near_token::NearToken;

use crate::{
    common::query::{
        CallResultHandler, MultiQueryBuilder, MultiQueryHandler, PostprocessHandler, QueryBuilder,
        QueryCreator, RpcValidatorHandler, SimpleQuery, SimpleValidatorRpc, ValidatorQueryBuilder,
        ViewStateHandler,
    },
    contract::Contract,
    transactions::ConstructTransaction,
    types::{
        stake::{RewardFeeFraction, StakingPoolInfo, UserStakeBalance},
        Data,
    },
};

fn near_data_to_near_token(data: Data<u128>) -> NearToken {
    NearToken::from_yoctonear(data.data)
}

#[derive(Clone, Debug)]
pub struct Delegation(pub AccountId);

impl Delegation {
    pub fn view_staked_balance(
        &self,
        pool: AccountId,
    ) -> anyhow::Result<QueryBuilder<PostprocessHandler<NearToken, CallResultHandler<u128>>>> {
        let args = serde_json::to_vec(&serde_json::json!({
            "account_id": self.0.clone(),
        }))?;
        let request = near_primitives::views::QueryRequest::CallFunction {
            account_id: pool,
            method_name: "get_account_staked_balance".to_owned(),
            args: near_primitives::types::FunctionArgs::from(args),
        };

        Ok(QueryBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            PostprocessHandler::new(
                CallResultHandler::default(),
                Box::new(near_data_to_near_token),
            ),
        ))
    }

    pub fn view_unstaked_balance(
        &self,
        pool: AccountId,
    ) -> anyhow::Result<QueryBuilder<PostprocessHandler<NearToken, CallResultHandler<u128>>>> {
        let args = serde_json::to_vec(&serde_json::json!({
            "account_id": self.0.clone(),
        }))?;
        let request = near_primitives::views::QueryRequest::CallFunction {
            account_id: pool,
            method_name: "get_account_unstaked_balance".to_owned(),
            args: near_primitives::types::FunctionArgs::from(args),
        };

        Ok(QueryBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            PostprocessHandler::new(
                CallResultHandler::default(),
                Box::new(near_data_to_near_token),
            ),
        ))
    }

    pub fn view_total_balance(
        &self,
        pool: AccountId,
    ) -> anyhow::Result<QueryBuilder<PostprocessHandler<NearToken, CallResultHandler<u128>>>> {
        let args = serde_json::to_vec(&serde_json::json!({
            "account_id": self.0.clone(),
        }))?;
        let request = near_primitives::views::QueryRequest::CallFunction {
            account_id: pool,
            method_name: "get_account_total_balance".to_owned(),
            args: near_primitives::types::FunctionArgs::from(args),
        };

        Ok(QueryBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            PostprocessHandler::new(
                CallResultHandler::default(),
                Box::new(near_data_to_near_token),
            ),
        ))
    }

    pub fn view_balance(
        &self,
        pool: AccountId,
    ) -> anyhow::Result<
        MultiQueryBuilder<
            PostprocessHandler<
                UserStakeBalance,
                MultiQueryHandler<(
                    CallResultHandler<u128>,
                    CallResultHandler<u128>,
                    CallResultHandler<u128>,
                )>,
            >,
        >,
    > {
        let postprocess = PostprocessHandler::new(
            MultiQueryHandler::new((
                CallResultHandler::default(),
                CallResultHandler::default(),
                CallResultHandler::default(),
            )),
            |(staked, unstaked, total)| {
                let staked = near_data_to_near_token(staked);
                let unstaked = near_data_to_near_token(unstaked);
                let total = near_data_to_near_token(total);

                UserStakeBalance {
                    staked,
                    unstaked,
                    total,
                }
            },
        );

        let multiquery = MultiQueryBuilder::new(postprocess, BlockReference::latest())
            .add_query_builder(self.view_staked_balance(pool.clone())?)
            .add_query_builder(self.view_staked_balance(pool.clone())?)
            .add_query_builder(self.view_total_balance(pool)?);

        Ok(multiquery)
    }

    pub fn is_account_unstaked_balance_available_for_withdrawal(
        &self,
        pool: AccountId,
    ) -> anyhow::Result<QueryBuilder<CallResultHandler<bool>>> {
        let args = serde_json::to_vec(&serde_json::json!({
            "account_id": self.0.clone(),
        }))?;

        let request = near_primitives::views::QueryRequest::CallFunction {
            account_id: pool,
            method_name: "is_account_unstaked_balance_available".to_owned(),
            args: near_primitives::types::FunctionArgs::from(args),
        };

        Ok(QueryBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            CallResultHandler::default(),
        ))
    }

    pub fn deposit(
        &self,
        pool: AccountId,
        amount: NearToken,
    ) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("deposit", ())?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .deposit(amount)
            .with_signer_account(self.0.clone()))
    }

    pub fn deposit_and_stake(
        &self,
        pool: AccountId,
        amount: NearToken,
    ) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("deposit_and_stake", ())?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .deposit(amount)
            .with_signer_account(self.0.clone()))
    }

    pub fn stake(
        &self,
        pool: AccountId,
        amount: NearToken,
    ) -> anyhow::Result<ConstructTransaction> {
        let args = serde_json::json!({
            "amount": amount.as_yoctonear(),
        });

        Ok(Contract(pool)
            .call_function("stake", args)?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone()))
    }

    pub fn stake_all(&self, pool: AccountId) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("stake_all", ())?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone()))
    }

    pub fn unstake(
        &self,
        pool: AccountId,
        amount: NearToken,
    ) -> anyhow::Result<ConstructTransaction> {
        let args = serde_json::json!({
            "amount": amount.as_yoctonear(),
        });

        Ok(Contract(pool)
            .call_function("unstake", args)?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone()))
    }

    pub fn unstake_all(&self, pool: AccountId) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("unstake_all", ())?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone()))
    }

    pub fn withdraw(
        &self,
        pool: AccountId,
        amount: NearToken,
    ) -> anyhow::Result<ConstructTransaction> {
        let args = serde_json::json!({
            "amount": amount.as_yoctonear(),
        });

        Ok(Contract(pool)
            .call_function("withdraw", args)?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone()))
    }

    pub fn withdraw_all(&self, pool: AccountId) -> anyhow::Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("withdraw_all", ())?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone()))
    }
}

#[derive(Clone, Debug)]
pub struct Staking {}

impl Staking {
    pub fn active_staking_pools(
    ) -> QueryBuilder<PostprocessHandler<std::collections::BTreeSet<AccountId>, ViewStateHandler>>
    {
        QueryBuilder::new(
            ActiveStakingPoolQuery,
            BlockReference::latest(),
            PostprocessHandler::new(ViewStateHandler, |query_result| {
                query_result
                    .data
                    .values
                    .into_iter()
                    .filter_map(|item| borsh::from_slice(&item.value).ok())
                    .collect()
            }),
        )
    }

    pub fn epoch_validators_info() -> ValidatorQueryBuilder<RpcValidatorHandler> {
        ValidatorQueryBuilder::new(
            SimpleValidatorRpc,
            EpochReference::Latest,
            RpcValidatorHandler,
        )
    }

    pub fn validators_stake() -> ValidatorQueryBuilder<
        PostprocessHandler<BTreeMap<AccountId, NearToken>, RpcValidatorHandler>,
    > {
        ValidatorQueryBuilder::new(
            SimpleValidatorRpc,
            EpochReference::Latest,
            PostprocessHandler::new(RpcValidatorHandler, |validator_response| {
                validator_response
                    .current_proposals
                    .into_iter()
                    .map(|validator_stake_view| {
                        let validator_stake = validator_stake_view.into_validator_stake();
                        validator_stake.account_and_stake()
                    })
                    .chain(validator_response.current_validators.into_iter().map(
                        |current_epoch_validator_info| {
                            (
                                current_epoch_validator_info.account_id,
                                current_epoch_validator_info.stake,
                            )
                        },
                    ))
                    .chain(validator_response.next_validators.into_iter().map(
                        |next_epoch_validator_info| {
                            (
                                next_epoch_validator_info.account_id,
                                next_epoch_validator_info.stake,
                            )
                        },
                    ))
                    .map(|(account_id, stake)| (account_id, NearToken::from_yoctonear(stake)))
                    .collect()
            }),
        )
    }

    pub fn staking_pool_reward_fee(
        pool: AccountId,
    ) -> QueryBuilder<CallResultHandler<RewardFeeFraction>> {
        Contract(pool)
            .call_function("get_reward_fee_fraction", ())
            .expect("arguments are not expected")
            .read_only()
    }

    pub fn staking_pool_delegators(pool: AccountId) -> QueryBuilder<CallResultHandler<u64>> {
        Contract(pool)
            .call_function("get_number_of_accounts", ())
            .expect("arguments are not expected")
            .read_only()
    }

    pub fn staking_pool_total_stake(
        pool: AccountId,
    ) -> QueryBuilder<PostprocessHandler<NearToken, CallResultHandler<u128>>> {
        let request = near_primitives::views::QueryRequest::CallFunction {
            account_id: pool,
            method_name: "get_total_staked_balance".to_owned(),
            args: near_primitives::types::FunctionArgs::from(vec![]),
        };

        QueryBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            PostprocessHandler::new(
                CallResultHandler::default(),
                Box::new(near_data_to_near_token),
            ),
        )
    }

    pub fn staking_pool_info(
        pool: AccountId,
    ) -> MultiQueryBuilder<
        PostprocessHandler<
            StakingPoolInfo,
            MultiQueryHandler<(
                CallResultHandler<RewardFeeFraction>,
                CallResultHandler<u64>,
                CallResultHandler<u128>,
            )>,
        >,
    > {
        let pool_clone = pool.clone();
        let postprocess = PostprocessHandler::new(
            MultiQueryHandler::new((
                CallResultHandler::default(),
                CallResultHandler::default(),
                CallResultHandler::default(),
            )),
            move |(reward_fee, delegators, total_stake)| {
                let total = near_data_to_near_token(total_stake);

                StakingPoolInfo {
                    validator_id: pool_clone.clone(),

                    fee: Some(reward_fee.data),
                    delegators: Some(delegators.data),
                    stake: total,
                }
            },
        );

        MultiQueryBuilder::new(postprocess, BlockReference::latest())
            .add_query_builder(Self::staking_pool_reward_fee(pool.clone()))
            .add_query_builder(Self::staking_pool_delegators(pool.clone()))
            .add_query_builder(Self::staking_pool_total_stake(pool))
    }

    pub fn delegation(account_id: AccountId) -> Delegation {
        Delegation(account_id)
    }
}

#[derive(Clone, Debug)]
pub struct ActiveStakingPoolQuery;

impl QueryCreator<RpcQueryRequest> for ActiveStakingPoolQuery {
    type RpcReference = BlockReference;

    fn create_query(
        &self,
        network: &crate::config::NetworkConfig,
        reference: Self::RpcReference,
    ) -> anyhow::Result<RpcQueryRequest> {
        Ok(RpcQueryRequest {
            block_reference: reference,
            request: near_primitives::views::QueryRequest::ViewState {
                account_id: network
                    .staking_pools_factory_account_id
                    .clone()
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Staking pools factory account ID is not set for the network"
                        )
                    })?,
                prefix: near_primitives::types::StoreKey::from(b"se".to_vec()),
                include_proof: false,
            },
        })
    }
}
