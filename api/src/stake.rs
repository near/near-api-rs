use std::collections::BTreeMap;

use near_openapi_client::types::{RpcError, RpcQueryResponse};
use near_api_types::{
    AccountId, Data, EpochReference, NearGas, NearToken, Reference,
    stake::{RewardFeeFraction, StakingPoolInfo, UserStakeBalance},
};

use crate::{
    NetworkConfig,
    advanced::{
        AndThenHandler, ResponseHandler, RpcBuilder, query_request::QueryRequest,
        query_rpc::SimpleQueryRpc, validator_rpc::SimpleValidatorRpc,
    },
    common::{
        query::{
            CallResultHandler, MultiQueryBuilder, MultiQueryHandler, PostprocessHandler,
            QueryBuilder, RpcType, RpcValidatorHandler, ValidatorQueryBuilder, ViewStateHandler,
        },
        utils::{from_base64, near_data_to_near_token, to_base64},
    },
    config::RetryResponse,
    contract::Contract,
    errors::{BuilderError, QueryCreationError, QueryError, SendRequestError},
    transactions::ConstructTransaction,
};

type Result<T> = core::result::Result<T, BuilderError>;

/// A wrapper struct that simplifies interactions with the [Staking Pool](https://github.com/near/core-contracts/tree/master/staking-pool) standard on behalf of the account.
///
/// Delegation is a wrapper that provides the functionality to manage user account stake in
/// the staking pool.
#[derive(Clone, Debug)]
pub struct Delegation(pub AccountId);

impl Delegation {
    /// Prepares a new contract query (`get_account_staked_balance`) for fetching the staked balance ([NearToken]) of the account on the staking pool.
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let balance = Staking::delegation("alice.testnet".parse()?)
    ///     .view_staked_balance("pool.testnet".parse()?)?
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Staked balance: {:?}", balance);
    /// # Ok(())
    /// # }
    /// ```
    pub fn view_staked_balance(
        &self,
        pool: AccountId,
    ) -> Result<QueryBuilder<PostprocessHandler<NearToken, CallResultHandler<u128>>>> {
        Ok(Contract(pool)
            .call_function(
                "get_account_staked_balance",
                serde_json::json!({
                    "account_id": self.0.clone(),
                }),
            )?
            .read_only()
            .map(near_data_to_near_token))
    }

    /// Prepares a new contract query (`get_account_unstaked_balance`) for fetching the unstaked(free, not used for staking) balance ([NearToken]) of the account on the staking pool.
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let balance = Staking::delegation("alice.testnet".parse()?)
    ///     .view_unstaked_balance("pool.testnet".parse()?)?
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Unstaked balance: {:?}", balance);
    /// # Ok(())
    /// # }
    /// ```
    pub fn view_unstaked_balance(
        &self,
        pool: AccountId,
    ) -> Result<QueryBuilder<PostprocessHandler<NearToken, CallResultHandler<u128>>>> {
        Ok(Contract(pool)
            .call_function(
                "get_account_unstaked_balance",
                serde_json::json!({
                    "account_id": self.0.clone(),
                }),
            )?
            .read_only()
            .map(near_data_to_near_token))
    }

    /// Prepares a new contract query (`get_account_total_balance`) for fetching the total balance ([NearToken]) of the account (free + staked) on the staking pool.
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let balance = Staking::delegation("alice.testnet".parse()?)
    ///     .view_total_balance("pool.testnet".parse()?)?
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Total balance: {:?}", balance);
    /// # Ok(())
    /// # }
    /// ```
    pub fn view_total_balance(
        &self,
        pool: AccountId,
    ) -> Result<QueryBuilder<PostprocessHandler<NearToken, CallResultHandler<u128>>>> {
        Ok(Contract(pool)
            .call_function(
                "get_account_total_balance",
                serde_json::json!({
                    "account_id": self.0.clone(),
                }),
            )?
            .read_only()
            .map(near_data_to_near_token))
    }

    /// Returns a full information about the staked balance ([UserStakeBalance]) of the account on the staking pool.
    ///
    /// This is a complex query that requires 3 calls (get_account_staked_balance, get_account_unstaked_balance, get_account_total_balance) to the staking pool contract.
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let balance = Staking::delegation("alice.testnet".parse()?)
    ///     .view_balance("pool.testnet".parse()?)?
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Balance: {:?}", balance);
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::type_complexity)]
    pub fn view_balance(
        &self,
        pool: AccountId,
    ) -> Result<
        MultiQueryBuilder<
            PostprocessHandler<
                UserStakeBalance,
                MultiQueryHandler<(
                    CallResultHandler<NearToken>,
                    CallResultHandler<NearToken>,
                    CallResultHandler<NearToken>,
                )>,
            >,
        >,
    > {
        let postprocess = MultiQueryHandler::default();

        let multiquery = MultiQueryBuilder::new(postprocess, Reference::Optimistic)
            .add_query_builder(self.view_staked_balance(pool.clone())?)
            .add_query_builder(self.view_unstaked_balance(pool.clone())?)
            .add_query_builder(self.view_total_balance(pool)?)
            .map(
                |(staked, unstaked, total): (Data<NearToken>, Data<NearToken>, Data<NearToken>)| {
                    UserStakeBalance {
                        staked: staked.data,
                        unstaked: unstaked.data,
                        total: total.data,
                    }
                },
            );
        Ok(multiquery)
    }

    /// Prepares a new contract query (`is_account_unstaked_balance_available`) for checking if the unstaked balance of the account is available for withdrawal.
    ///
    /// Some pools configures minimum withdrawal period in epochs, so the balance is not available for withdrawal immediately.
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let is_available = Staking::delegation("alice.testnet".parse()?)
    ///     .is_account_unstaked_balance_available_for_withdrawal("pool.testnet".parse()?)?
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Is available: {:?}", is_available);
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_account_unstaked_balance_available_for_withdrawal(
        &self,
        pool: AccountId,
    ) -> Result<QueryBuilder<CallResultHandler<bool>>> {
        Ok(Contract(pool)
            .call_function(
                "is_account_unstaked_balance_available",
                serde_json::json!({
                    "account_id": self.0.clone(),
                }),
            )?
            .read_only())
    }

    /// Prepares a new transaction contract call (`deposit`) for depositing funds into the staking pool.
    /// Please note that your deposit is not staked, and it will be allocated as unstaked (free) balance.
    ///
    /// Please note that this call will deposit your account tokens into the contract, so you will not be able to use them for other purposes.
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let result = Staking::delegation("alice.testnet".parse()?)
    ///     .deposit("pool.testnet".parse()?, NearToken::from_near(1))?
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn deposit(&self, pool: AccountId, amount: NearToken) -> Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("deposit", ())?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .deposit(amount)
            .with_signer_account(self.0.clone()))
    }

    /// Prepares a new transaction contract call (`deposit_and_stake`) for depositing funds into the staking pool and staking them.
    ///
    /// Please note that this call will deposit your account tokens into the contract, so you will not be able to use them for other purposes.
    /// Also, after you have staked your funds, if you decide to withdraw them, you might need to wait for the two lockup period to end.
    /// * Mandatory lockup before able to unstake
    /// * Optional lockup before able to withdraw (depends on the pool configuration)
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let result = Staking::delegation("alice.testnet".parse()?)
    ///     .deposit_and_stake("pool.testnet".parse()?, NearToken::from_near(1))?
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn deposit_and_stake(
        &self,
        pool: AccountId,
        amount: NearToken,
    ) -> Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("deposit_and_stake", ())?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .deposit(amount)
            .with_signer_account(self.0.clone()))
    }

    /// Prepares a new transaction contract call (`stake`) for staking funds into the staking pool.
    ///
    /// Please note that this call will use your unstaked balance. This means that you have to have enough balance already deposited into the contract.
    /// This won't use your native account tokens, but just reallocate your balance inside the contract.
    /// Please also be aware that once you have staked your funds, you might not be able to withdraw them until the lockup periods end.
    /// * Mandatory lockup before able to unstake
    /// * Optional lockup before able to withdraw (depends on the pool configuration)
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let result = Staking::delegation("alice.testnet".parse()?)
    ///     .stake("pool.testnet".parse()?, NearToken::from_near(1))?
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn stake(&self, pool: AccountId, amount: NearToken) -> Result<ConstructTransaction> {
        let args = serde_json::json!({
            "amount": amount.as_yoctonear(),
        });

        Ok(Contract(pool)
            .call_function("stake", args)?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone()))
    }

    /// Prepares a new transaction contract call (`stake_all`) for staking all available unstaked balance into the staking pool.
    ///
    /// Please note that once you have staked your funds, you might not be able to withdraw them until the lockup periods end.
    /// * Mandatory lockup before able to unstake
    /// * Optional lockup before able to withdraw (depends on the pool configuration)
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Staking::delegation("alice.testnet".parse()?)
    ///     .stake_all("pool.testnet".parse()?)?
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn stake_all(&self, pool: AccountId) -> Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("stake_all", ())?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone()))
    }

    /// Prepares a new transaction contract call (`unstake`) for unstaking funds and returning them to your unstaked balance.
    ///
    /// The contract will check if the minimum epoch height condition is met.
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let result = Staking::delegation("alice.testnet".parse()?)
    ///     .unstake("pool.testnet".parse()?, NearToken::from_near(1))?
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn unstake(&self, pool: AccountId, amount: NearToken) -> Result<ConstructTransaction> {
        let args = serde_json::json!({
            "amount": amount.as_yoctonear(),
        });

        Ok(Contract(pool)
            .call_function("unstake", args)?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone()))
    }

    /// Prepares a new transaction contract call (`unstake_all`) for unstaking all available staked balance and returning them to your unstaked balance.
    ///
    /// The contract will check if the minimum epoch height condition is met.
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let result = Staking::delegation("alice.testnet".parse()?)
    ///     .unstake_all("pool.testnet".parse()?)?
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn unstake_all(&self, pool: AccountId) -> Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("unstake_all", ())?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone()))
    }

    /// Prepares a new transaction contract call (`withdraw`) for withdrawing funds from the staking pool into your account.
    ///
    /// Some pools configures minimum withdrawal period in epochs, so the balance is not available for withdrawal immediately.
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let result = Staking::delegation("alice.testnet".parse()?)
    ///     .withdraw("pool.testnet".parse()?, NearToken::from_near(1))?
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn withdraw(&self, pool: AccountId, amount: NearToken) -> Result<ConstructTransaction> {
        let args = serde_json::json!({
            "amount": amount.as_yoctonear(),
        });

        Ok(Contract(pool)
            .call_function("withdraw", args)?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone()))
    }

    /// Prepares a new transaction contract call (`withdraw_all`) for withdrawing all available staked balance from the staking pool into your account.
    ///
    /// Some pools configures minimum withdrawal period in epochs, so the balance is not available for withdrawal immediately.
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let result = Staking::delegation("alice.testnet".parse()?)
    ///     .withdraw_all("pool.testnet".parse()?)?
    ///     .with_signer(Signer::new(Signer::from_ledger())?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn withdraw_all(&self, pool: AccountId) -> Result<ConstructTransaction> {
        Ok(Contract(pool)
            .call_function("withdraw_all", ())?
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone()))
    }
}

/// Staking-related interactions with the NEAR Protocol and the staking pools.
///
/// The [`Staking`] struct provides methods to interact with NEAR staking, including querying staking pools, validators, and delegators,
/// as well as delegating and withdrawing from staking pools.
///
/// # Examples
///
/// ```rust,no_run
/// use near_api::*;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let staking_pools = Staking::active_staking_pools().fetch_from_testnet().await?;
/// println!("Staking pools: {:?}", staking_pools);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct Staking {}

impl Staking {
    /// Returns a list of active staking pools ([std::collections::BTreeSet]<[AccountId]>]) by querying the staking pools factory contract.
    ///
    /// Please note that it might fail on the mainnet as the staking pool factory is super huge.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let staking_pools = Staking::active_staking_pools().fetch_from_testnet().await?;
    /// println!("Staking pools: {:?}", staking_pools);
    /// # Ok(())
    /// # }
    /// ```
    pub fn active_staking_pools() -> RpcBuilder<ActiveStakingPoolQuery, ActiveStakingHandler> {
        RpcBuilder::new(
            ActiveStakingPoolQuery,
            Reference::Optimistic,
            ActiveStakingHandler,
        )
    }

    /// Returns a list of validators and their stake ([near_api_types::RpcValidatorResponse]) for the current epoch.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let validators = Staking::epoch_validators_info().fetch_from_testnet().await?;
    /// println!("Validators: {:?}", validators);
    /// # Ok(())
    /// # }
    /// ```
    pub fn epoch_validators_info() -> ValidatorQueryBuilder<RpcValidatorHandler> {
        ValidatorQueryBuilder::new(
            SimpleValidatorRpc,
            EpochReference::Latest,
            RpcValidatorHandler,
        )
    }

    /// Returns a map of validators and their stake ([BTreeMap<AccountId, NearToken>]) for the current epoch.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let validators = Staking::validators_stake().fetch_from_testnet().await?;
    /// println!("Validators: {:?}", validators);
    /// # Ok(())
    /// # }
    /// ```
    pub fn validators_stake()
    -> ValidatorQueryBuilder<AndThenHandler<BTreeMap<AccountId, NearToken>, RpcValidatorHandler>>
    {
        ValidatorQueryBuilder::new(
            SimpleValidatorRpc,
            EpochReference::Latest,
            RpcValidatorHandler,
        )
        .and_then(|validator_response| {
            validator_response
                .current_proposals
                .into_iter()
                .map(|validator_stake_view| {
                    (validator_stake_view.account_id, validator_stake_view.stake)
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
                .map(|(account_id, stake)| {
                    Ok((account_id, NearToken::from_yoctonear(stake.parse()?)))
                })
                .collect::<::core::result::Result<_, Box<dyn std::error::Error + Send + Sync>>>()
        })
    }

    /// Prepares a new contract query (`get_reward_fee_fraction`) for fetching the reward fee fraction of the staking pool ([Data]<[RewardFeeFraction]>).
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let reward_fee = Staking::staking_pool_reward_fee("pool.testnet".parse()?)
    /// .fetch_from_testnet().await?;
    /// println!("Reward fee: {:?}", reward_fee);
    /// # Ok(())
    /// # }
    /// ```
    pub fn staking_pool_reward_fee(
        pool: AccountId,
    ) -> QueryBuilder<CallResultHandler<RewardFeeFraction>> {
        Contract(pool)
            .call_function("get_reward_fee_fraction", ())
            .expect("arguments are not expected")
            .read_only()
    }

    /// Prepares a new contract query (`get_number_of_accounts`) for fetching the number of delegators of the staking pool ([Data]<[u64]>).
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let delegators = Staking::staking_pool_delegators("pool.testnet".parse()?)
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Delegators: {:?}", delegators);
    /// # Ok(())
    /// # }
    /// ```
    pub fn staking_pool_delegators(pool: AccountId) -> QueryBuilder<CallResultHandler<u64>> {
        Contract(pool)
            .call_function("get_number_of_accounts", ())
            .expect("arguments are not expected")
            .read_only()
    }

    /// Prepares a new contract query (`get_total_staked_balance`) for fetching the total stake of the staking pool ([NearToken]).
    ///
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let total_stake = Staking::staking_pool_total_stake("pool.testnet".parse()?)
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Total stake: {:?}", total_stake);
    /// # Ok(())
    /// # }
    /// ```
    pub fn staking_pool_total_stake(
        pool: AccountId,
    ) -> QueryBuilder<PostprocessHandler<NearToken, CallResultHandler<u128>>> {
        Contract(pool)
            .call_function("get_total_staked_balance", ())
            .expect("arguments are not expected")
            .read_only()
            .map(near_data_to_near_token)
    }

    /// Returns a full information about the staking pool ([StakingPoolInfo]).
    ///
    /// This is a complex query that requires 3 calls (get_reward_fee_fraction, get_number_of_accounts, get_total_staked_balance) to the staking pool contract.
    /// The call depends that the contract implements [`StakingPool`](https://github.com/near/core-contracts/tree/master/staking-pool)
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let staking_pool_info = Staking::staking_pool_info("pool.testnet".parse()?)
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Staking pool info: {:?}", staking_pool_info);
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::type_complexity)]
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
        let handler = MultiQueryHandler::new((
            CallResultHandler::default(),
            CallResultHandler::default(),
            CallResultHandler::default(),
        ));

        MultiQueryBuilder::new(handler, Reference::Optimistic)
            .add_query_builder(Self::staking_pool_reward_fee(pool.clone()))
            .add_query_builder(Self::staking_pool_delegators(pool.clone()))
            .add_query_builder(Self::staking_pool_total_stake(pool))
            .map(move |(reward_fee, delegators, total_stake)| {
                let total = near_data_to_near_token(total_stake);

                StakingPoolInfo {
                    validator_id: pool_clone.clone(),

                    fee: Some(reward_fee.data),
                    delegators: Some(delegators.data),
                    stake: total,
                }
            })
    }

    /// Returns a new [`Delegation`] struct for interacting with the staking pool on behalf of the account.
    pub const fn delegation(account_id: AccountId) -> Delegation {
        Delegation(account_id)
    }
}

#[derive(Clone, Debug)]
pub struct ActiveStakingPoolQuery;

#[async_trait::async_trait]
impl RpcType for ActiveStakingPoolQuery {
    type RpcReference = <SimpleQueryRpc as RpcType>::RpcReference;
    type Response = <SimpleQueryRpc as RpcType>::Response;
    type Error = <SimpleQueryRpc as RpcType>::Error;

    async fn send_query(
        &self,
        client: &near_openapi_client::Client,
        network: &NetworkConfig,
        reference: &Reference,
    ) -> RetryResponse<RpcQueryResponse, SendRequestError<RpcError>> {
        let Some(account_id) = network.staking_pools_factory_account_id.clone() else {
            return RetryResponse::Critical(SendRequestError::QueryCreationError(
                QueryCreationError::StakingPoolFactoryNotDefined,
            ));
        };

        let request = QueryRequest::ViewState {
            account_id,
            prefix_base64: near_api_types::StoreKey(to_base64(b"se")),
            include_proof: Some(false),
        };

        SimpleQueryRpc { request }
            .send_query(client, network, reference)
            .await
    }
}

#[derive(Clone, Debug)]
pub struct ActiveStakingHandler;

#[async_trait::async_trait]
impl ResponseHandler for ActiveStakingHandler {
    type Query = ActiveStakingPoolQuery;
    type Response = std::collections::BTreeSet<AccountId>;

    fn process_response(
        &self,
        response: Vec<RpcQueryResponse>,
    ) -> core::result::Result<Self::Response, QueryError<RpcError>> {
        let query_result = ViewStateHandler {}.process_response(response)?;

        Ok(query_result
            .data
            .values
            .into_iter()
            .filter_map(|item| borsh::from_slice(&from_base64(&item.value).ok()?).ok())
            .collect())
    }
}
