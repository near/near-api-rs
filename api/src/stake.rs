use std::collections::BTreeMap;

use near_api_types::{
    stake::{RewardFeeFraction, StakingPoolInfo, UserStakeBalance},
    AccountId, Data, EpochReference, NearGas, NearToken, Reference,
};
use near_openapi_client::types::{RpcQueryError, RpcQueryResponse};

use crate::{
    advanced::{
        query_request::QueryRequest, query_rpc::SimpleQueryRpc, validator_rpc::SimpleValidatorRpc,
        ResponseHandler, RpcBuilder,
    },
    common::{
        query::{
            CallResultHandler, MultiQueryHandler, MultiRequestBuilder, PostprocessHandler,
            RequestBuilder, RpcType, RpcValidatorHandler, ViewStateHandler,
        },
        utils::{from_base64, near_data_to_near_token, to_base64},
    },
    config::RetryResponse,
    contract::Contract,
    errors::{QueryCreationError, QueryError, SendRequestError},
    transactions::ConstructTransaction,
    NetworkConfig,
};

/// A wrapper struct that simplifies interactions with the [Staking Pool](https://github.com/near/core-contracts/tree/master/staking-pool) standard on behalf of the account.
///
/// Delegation is a wrapper that provides the functionality to manage user account stake in
/// the staking pool.
#[derive(Clone, Debug)]
pub struct Delegation(pub AccountId);

impl Delegation {
    /// Returns the underlying account ID for this delegation.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let delegation = Staking::delegation("alice.testnet".parse()?);
    /// let account_id = delegation.account_id();
    /// println!("Account ID: {}", account_id);
    /// # Ok(())
    /// # }
    /// ```
    pub const fn account_id(&self) -> &AccountId {
        &self.0
    }

    /// Converts this delegation to an Account for account-related operations.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let delegation = Staking::delegation("alice.testnet".parse()?);
    /// let account = delegation.as_account();
    /// let account_info = account.view().fetch_from_testnet().await?;
    /// println!("Account balance: {}", account_info.data.amount);
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_account(&self) -> crate::account::Account {
        crate::account::Account::from_id(&self.0)
    }

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
    ///     .view_staked_balance("pool.testnet".parse()?)
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Staked balance: {:?}", balance);
    /// # Ok(())
    /// # }
    /// ```
    pub fn view_staked_balance(
        &self,
        pool: AccountId,
    ) -> RequestBuilder<PostprocessHandler<NearToken, CallResultHandler<u128>>> {
        Contract::from_id(pool)
            .call_function(
                "get_account_staked_balance",
                serde_json::json!({
                    "account_id": self.0.clone(),
                }),
            )
            .read_only()
            .map(near_data_to_near_token)
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
    ///     .view_unstaked_balance("pool.testnet".parse()?)
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Unstaked balance: {:?}", balance);
    /// # Ok(())
    /// # }
    /// ```
    pub fn view_unstaked_balance(
        &self,
        pool: AccountId,
    ) -> RequestBuilder<PostprocessHandler<NearToken, CallResultHandler<u128>>> {
        Contract::from_id(pool)
            .call_function(
                "get_account_unstaked_balance",
                serde_json::json!({
                    "account_id": self.0.clone(),
                }),
            )
            .read_only()
            .map(near_data_to_near_token)
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
    ///     .view_total_balance("pool.testnet".parse()?)
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Total balance: {:?}", balance);
    /// # Ok(())
    /// # }
    /// ```
    pub fn view_total_balance(
        &self,
        pool: AccountId,
    ) -> RequestBuilder<PostprocessHandler<NearToken, CallResultHandler<u128>>> {
        Contract::from_id(pool)
            .call_function(
                "get_account_total_balance",
                serde_json::json!({
                    "account_id": self.0.clone(),
                }),
            )
            .read_only()
            .map(near_data_to_near_token)
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
    ///     .view_balance("pool.testnet".parse()?)
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
    ) -> MultiRequestBuilder<
        PostprocessHandler<
            UserStakeBalance,
            MultiQueryHandler<(
                CallResultHandler<NearToken>,
                CallResultHandler<NearToken>,
                CallResultHandler<NearToken>,
            )>,
        >,
    > {
        let postprocess = MultiQueryHandler::default();

        MultiRequestBuilder::new(postprocess, Reference::Optimistic)
            .add_query_builder(self.view_staked_balance(pool.clone()))
            .add_query_builder(self.view_unstaked_balance(pool.clone()))
            .add_query_builder(self.view_total_balance(pool))
            .map(
                |(staked, unstaked, total): (Data<NearToken>, Data<NearToken>, Data<NearToken>)| {
                    UserStakeBalance {
                        staked: staked.data,
                        unstaked: unstaked.data,
                        total: total.data,
                    }
                },
            )
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
    ///     .is_account_unstaked_balance_available_for_withdrawal("pool.testnet".parse()?)
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Is available: {:?}", is_available);
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_account_unstaked_balance_available_for_withdrawal(
        &self,
        pool: AccountId,
    ) -> RequestBuilder<CallResultHandler<bool>> {
        Contract::from_id(pool)
            .call_function(
                "is_account_unstaked_balance_available",
                serde_json::json!({
                    "account_id": self.0.clone(),
                }),
            )
            .read_only()
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
    ///     .deposit("pool.testnet".parse()?, NearToken::from_near(1))
    ///     .with_signer(Signer::from_ledger()?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn deposit(&self, pool: AccountId, amount: NearToken) -> ConstructTransaction {
        Contract::from_id(pool)
            .call_function("deposit", ())
            .transaction()
            .gas(NearGas::from_tgas(50))
            .deposit(amount)
            .with_signer_account(self.0.clone())
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
    ///     .deposit_and_stake("pool.testnet".parse()?, NearToken::from_near(1))
    ///     .with_signer(Signer::from_ledger()?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn deposit_and_stake(&self, pool: AccountId, amount: NearToken) -> ConstructTransaction {
        Contract::from_id(pool)
            .call_function("deposit_and_stake", ())
            .transaction()
            .gas(NearGas::from_tgas(50))
            .deposit(amount)
            .with_signer_account(self.0.clone())
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
    ///     .stake("pool.testnet".parse()?, NearToken::from_near(1))
    ///     .with_signer(Signer::from_ledger()?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn stake(&self, pool: AccountId, amount: NearToken) -> ConstructTransaction {
        let args = serde_json::json!({
            "amount": amount,
        });

        Contract::from_id(pool)
            .call_function("stake", args)
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone())
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
    ///     .stake_all("pool.testnet".parse()?)
    ///     .with_signer(Signer::from_ledger()?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn stake_all(&self, pool: AccountId) -> ConstructTransaction {
        Contract::from_id(pool)
            .call_function("stake_all", ())
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone())
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
    ///     .unstake("pool.testnet".parse()?, NearToken::from_near(1))
    ///     .with_signer(Signer::from_ledger()?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn unstake(&self, pool: AccountId, amount: NearToken) -> ConstructTransaction {
        let args = serde_json::json!({
            "amount": amount,
        });

        Contract::from_id(pool)
            .call_function("unstake", args)
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone())
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
    ///     .unstake_all("pool.testnet".parse()?)
    ///     .with_signer(Signer::from_ledger()?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn unstake_all(&self, pool: AccountId) -> ConstructTransaction {
        Contract::from_id(pool)
            .call_function("unstake_all", ())
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone())
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
    ///     .withdraw("pool.testnet".parse()?, NearToken::from_near(1))
    ///     .with_signer(Signer::from_ledger()?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn withdraw(&self, pool: AccountId, amount: NearToken) -> ConstructTransaction {
        let args = serde_json::json!({
            "amount": amount,
        });

        Contract::from_id(pool)
            .call_function("withdraw", args)
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone())
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
    ///     .withdraw_all("pool.testnet".parse()?)
    ///     .with_signer(Signer::from_ledger()?)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn withdraw_all(&self, pool: AccountId) -> ConstructTransaction {
        Contract::from_id(pool)
            .call_function("withdraw_all", ())
            .transaction()
            .gas(NearGas::from_tgas(50))
            .with_signer_account(self.0.clone())
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
            Ok(ActiveStakingPoolQuery),
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
    pub fn epoch_validators_info() -> RequestBuilder<RpcValidatorHandler> {
        RequestBuilder::new(
            Ok(SimpleValidatorRpc),
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
    pub fn validators_stake(
    ) -> RequestBuilder<PostprocessHandler<BTreeMap<AccountId, NearToken>, RpcValidatorHandler>>
    {
        RequestBuilder::new(
            Ok(SimpleValidatorRpc),
            EpochReference::Latest,
            RpcValidatorHandler,
        )
        .map(|validator_response| {
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
                .collect::<BTreeMap<_, NearToken>>()
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
    ) -> RequestBuilder<CallResultHandler<RewardFeeFraction>> {
        Contract::from_id(pool)
            .call_function("get_reward_fee_fraction", ())
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
    pub fn staking_pool_delegators(pool: AccountId) -> RequestBuilder<CallResultHandler<u64>> {
        Contract::from_id(pool)
            .call_function("get_number_of_accounts", ())
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
    ) -> RequestBuilder<PostprocessHandler<NearToken, CallResultHandler<u128>>> {
        Contract::from_id(pool)
            .call_function("get_total_staked_balance", ())
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
    ) -> MultiRequestBuilder<
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

        MultiRequestBuilder::new(handler, Reference::Optimistic)
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
    ) -> RetryResponse<RpcQueryResponse, SendRequestError<RpcQueryError>> {
        let Some(account_id) = network.staking_pools_factory_account_id.clone() else {
            return RetryResponse::Critical(SendRequestError::RequestCreationError(
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
    ) -> core::result::Result<Self::Response, QueryError<RpcQueryError>> {
        let query_result = ViewStateHandler {}.process_response(response)?;

        Ok(query_result
            .data
            .values
            .into_iter()
            .filter_map(|item| borsh::from_slice(&from_base64(&item.value).ok()?).ok())
            .collect())
    }
}
