use anyhow::bail;
use near_primitives::types::{AccountId, BlockReference};
use serde::de::DeserializeOwned;

use crate::Client;

pub struct ContractHandler<'client> {
    client: &'client Client,
    contract_id: AccountId,
}

impl<'client> ContractHandler<'client> {
    pub fn new(client: &'client Client, contract_id: AccountId) -> Self {
        Self {
            client,
            contract_id,
        }
    }

    pub async fn view<Args, Response>(
        &self,
        method_name: &str,
        args: Args,
    ) -> anyhow::Result<(Response, Vec<String>)>
    where
        Args: serde::Serialize,
        Response: DeserializeOwned,
    {
        let args = serde_json::to_vec(&args)?;
        let query = near_jsonrpc_client::methods::query::RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: near_primitives::views::QueryRequest::CallFunction {
                account_id: self.contract_id.clone(),
                method_name: method_name.to_owned(),
                args: near_primitives::types::FunctionArgs::from(args),
            },
        };
        let call_result =
            if let near_jsonrpc_primitives::types::query::QueryResponseKind::CallResult(result) =
                self.client.json_rpc_client.call(query).await?.kind
            {
                result
            } else {
                bail!("Received unexpected query kind in response to a view-function query call")
            };

        let result: Vec<u8> = call_result.result;
        let response: Response = serde_json::from_slice(&result)?;

        Ok((response, call_result.logs))
    }
}

#[cfg(test)]
mod tests {
    use crate::{config::Config, Client};

    #[derive(serde::Serialize)]
    pub struct Paging {
        limit: u32,
        page: u32,
    }

    #[tokio::test]
    async fn fetch_from_contract() {
        let config = Config::default();
        let client = Client::with_config(config.network_connection["testnet"].clone());
        let account = client.contract("race-of-sloths-stage.testnet".parse().unwrap());
        let result: (serde_json::Value, Vec<String>) = account
            .view("prs", Paging { limit: 5, page: 1 })
            .await
            .unwrap();

        assert!(result.0.is_array());
    }
}
