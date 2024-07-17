use near_token::NearToken;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakingPoolInfo {
    pub validator_id: near_primitives::types::AccountId,
    pub fee: Option<RewardFeeFraction>,
    pub delegators: Option<u64>,
    pub stake: NearToken,
}

#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct RewardFeeFraction {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct UserStakeBalance {
    pub staked: NearToken,
    pub unstaked: NearToken,
    pub total: NearToken,
}
