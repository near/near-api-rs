use near_primitives::types::AccountId;

use crate::query::{QueryBuilder, ViewStateHandler};

pub struct Staking {}

impl Staking {
    pub fn staking_pools(
        &self,
        factory: AccountId,
    ) -> QueryBuilder<ViewStateHandler<std::collections::BTreeSet<AccountId>>> {
        let request = near_primitives::views::QueryRequest::ViewState {
            account_id: factory,
            prefix: near_primitives::types::StoreKey::from(b"se".to_vec()),
            include_proof: false,
        };

        QueryBuilder::new(
            request,
            ViewStateHandler::with_postprocess(|query_result| {
                query_result
                    .values
                    .into_iter()
                    .flat_map(|item| String::from_utf8(item.value.into()))
                    .flat_map(|result| result.parse())
                    .collect()
            }),
        )
    }
}

#[cfg(test)]
mod tests {

    use crate::config::NetworkConfig;

    #[tokio::test]
    async fn get_pools() {
        let staking = super::Staking {};
        let pools = staking
            .staking_pools(
                NetworkConfig::mainnet()
                    .staking_pools_factory_account_id
                    .unwrap(),
            )
            .fetch_from_mainnet()
            .await
            .unwrap();

        for pool in pools.data.iter() {
            println!("{}", pool);
        }
    }
}
