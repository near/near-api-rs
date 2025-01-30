use near_token::NearToken;
use serde::{Deserialize, Serialize};

/// Aggregate information about the staking pool.
///
/// The type is related to the [StakingPool](https://github.com/near/core-contracts/tree/master/staking-pool) smart contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingPoolInfo {
    /// The validator that is running the pool.
    pub validator_id: near_primitives::types::AccountId,
    /// The fee that is taken by the pool contract.
    pub fee: Option<RewardFeeFraction>,
    /// The number of delegators on the pool.
    pub delegators: Option<u64>,
    /// The total staked balance on the pool (by all delegators).
    pub stake: NearToken,
}

/// The reward fee that is taken by the pool contract.
///
/// This represents the percentage of the reward that is taken by the pool contract.
/// The type is a part of the [StakingPool](https://github.com/near/core-contracts/tree/master/staking-pool) interface
///
/// The fraction is equal to numerator/denominator, e.g. 3/1000 = 0.3%
#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct RewardFeeFraction {
    /// The numerator of the fraction.
    pub numerator: u32,
    /// The denominator of the fraction.
    pub denominator: u32,
}

/// The total user balance on a pool contract
///
/// The type is related to the [StakingPool](https://github.com/near/core-contracts/tree/master/staking-pool) smart contract.
#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct UserStakeBalance {
    /// The balance that currently is staked. The user can't withdraw this balance until `unstake` is called
    /// and withdraw period is over.
    pub staked: NearToken,
    /// The balance that is not staked. The user can start withdrawing this balance. Some pools
    /// have a withdraw period.
    pub unstaked: NearToken,
    /// The total balance of the user on a contract (staked + unstaked)
    pub total: NearToken,
}
