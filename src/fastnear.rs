// use std::collections::BTreeSet;

// use near_primitives::types::AccountId;

// use crate::{types::StakingResponse, Client};

// pub struct FastNearHandler<'client> {
//     client: &'client Client,
//     fastnear_url: url::Url,
// }

// impl<'client> FastNearHandler<'client> {
//     pub fn new(client: &'client Client) -> anyhow::Result<Self> {
//         if client.config.fastnear_url.is_none() {
//             return Err(anyhow::anyhow!("FastNear URL is not set"));
//         }

//         Ok(Self {
//             client,
//             fastnear_url: client.config.fastnear_url.clone().unwrap(),
//         })
//     }

//     pub async fn account_delegated_in(
//         &self,
//         account_id: &AccountId,
//     ) -> anyhow::Result<BTreeSet<AccountId>> {
//         let request = reqwest::get(
//             self.fastnear_url
//                 .join(&format!("v1/account/{}/staking", account_id))?,
//         )
//         .await?;
//         let response: StakingResponse = request.json().await?;

//         Ok(response
//             .pools
//             .into_iter()
//             .map(|pool| pool.pool_id)
//             .collect())
//     }
// }
