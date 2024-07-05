// use near_primitives::types::AccountId;

// pub struct Staking {}

// impl Staking {
//     pub fn new() -> Self {
//         Self {}
//     }

//     pub async fn staking_pools(&self) -> anyhow::Result<std::collections::BTreeSet<AccountId>> {
//         let query_view_method_response = self
//             .client
//             .json_rpc_client
//             .call(near_jsonrpc_client::methods::query::RpcQueryRequest {
//                 block_reference: near_primitives::types::Finality::Final.into(),
//                 request: near_primitives::views::QueryRequest::ViewState {
//                     account_id: self.staking_pool_factory.clone(),
//                     prefix: near_primitives::types::StoreKey::from(Vec::new()),
//                     include_proof: false,
//                 },
//             })
//             .await
//             .context(format!(
//                 "Failed to fetch query ViewState for {} on the selected network",
//                 self.staking_pool_factory
//             ))?;
//         if let near_jsonrpc_primitives::types::query::QueryResponseKind::ViewState(result) =
//             query_view_method_response.kind
//         {
//             Ok(result
//                 .values
//                 .into_iter()
//                 .filter(|item| &item.key[..2] == b"se")
//                 .filter_map(|item| String::from_utf8(item.value.into()).ok())
//                 .filter_map(|result| result.parse().ok())
//                 .collect())
//         } else {
//             Err(anyhow::anyhow!(
//                 "Unexpected response: {:#?}",
//                 query_view_method_response
//             ))
//         }
//     }
// }
