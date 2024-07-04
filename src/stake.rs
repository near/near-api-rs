use anyhow::Context;
use near_primitives::types::AccountId;

pub struct StakingHandler<'client> {
    client: &'client super::Client,
    staking_pool_factory: AccountId,
}

impl<'client> StakingHandler<'client> {
    pub fn new(client: &'client super::Client) -> anyhow::Result<Self> {
        if client.config.staking_pools_factory_account_id.is_none() {
            return Err(anyhow::anyhow!(
                "Staking pools factory account ID is not set"
            ));
        }

        Ok(Self {
            client,
            staking_pool_factory: client
                .config
                .staking_pools_factory_account_id
                .clone()
                .unwrap(),
        })
    }

    pub async fn get_staking_pools_from_staking_pool_factory(
        &self,
    ) -> anyhow::Result<std::collections::BTreeSet<AccountId>> {
        let query_view_method_response = self
            .client
            .json_rpc_client
            .call(near_jsonrpc_client::methods::query::RpcQueryRequest {
                block_reference: near_primitives::types::Finality::Final.into(),
                request: near_primitives::views::QueryRequest::ViewState {
                    account_id: self.staking_pool_factory.clone(),
                    prefix: near_primitives::types::StoreKey::from(Vec::new()),
                    include_proof: false,
                },
            })
            .await
            .context(format!(
                "Failed to fetch query ViewState for {} on the selected network",
                self.staking_pool_factory
            ))?;
        if let near_jsonrpc_primitives::types::query::QueryResponseKind::ViewState(result) =
            query_view_method_response.kind
        {
            Ok(result
                .values
                .into_iter()
                .filter(|item| &item.key[..2] == b"se")
                .filter_map(|item| String::from_utf8(item.value.into()).ok())
                .filter_map(|result| result.parse().ok())
                .collect())
        } else {
            Err(anyhow::anyhow!(
                "Unexpected response: {:#?}",
                query_view_method_response
            ))
        }
    }
}
