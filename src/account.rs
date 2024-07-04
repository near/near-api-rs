use std::collections::BTreeMap;

use anyhow::bail;
use futures::{StreamExt, TryStreamExt};
use near_primitives::{
    types::{AccountId, Finality},
    views::{AccessKeyList, AccountView},
};
use near_token::NearToken;

pub struct AccountHandler<'client> {
    client: &'client super::Client,
    account_id: AccountId,
}

impl<'client> AccountHandler<'client> {
    pub fn new(client: &'client super::Client, account_id: AccountId) -> Self {
        Self { client, account_id }
    }

    pub async fn account(&self) -> anyhow::Result<AccountView> {
        let query_response = self
            .client
            .json_rpc_client
            .call(near_jsonrpc_client::methods::query::RpcQueryRequest {
                block_reference: near_primitives::types::BlockReference::Finality(Finality::Final),
                request: near_primitives::views::QueryRequest::ViewAccount {
                    account_id: self.account_id.clone(),
                },
            })
            .await?;

        if let near_jsonrpc_primitives::types::query::QueryResponseKind::ViewAccount(account) =
            query_response.kind
        {
            Ok(account)
        } else {
            Err(anyhow::anyhow!(
                "Unexpected response: {:#?}",
                query_response
            ))
        }
    }

    pub async fn list_keys(&self) -> anyhow::Result<AccessKeyList> {
        let query_response = self
            .client
            .json_rpc_client
            .call(near_jsonrpc_client::methods::query::RpcQueryRequest {
                block_reference: near_primitives::types::BlockReference::Finality(Finality::Final),
                request: near_primitives::views::QueryRequest::ViewAccessKeyList {
                    account_id: self.account_id.clone(),
                },
            })
            .await?;

        if let near_jsonrpc_primitives::types::query::QueryResponseKind::AccessKeyList(
            access_key_list,
        ) = query_response.kind
        {
            Ok(access_key_list)
        } else {
            Err(anyhow::anyhow!(
                "Unexpected response: {:#?}",
                query_response
            ))
        }
    }

    pub async fn delegation_in_pool(&self, pool: &AccountId) -> anyhow::Result<NearToken> {
        let account_staked_balance_response = self
            .client
            .json_rpc_client
            .call(near_jsonrpc_client::methods::query::RpcQueryRequest {
                block_reference: near_primitives::types::Finality::Final.into(),
                request: near_primitives::views::QueryRequest::CallFunction {
                    account_id: pool.clone(),
                    method_name: "get_account_staked_balance".to_string(),
                    args: near_primitives::types::FunctionArgs::from(serde_json::to_vec(
                        &serde_json::json!({
                            "account_id": self.account_id.clone(),
                        }),
                    )?),
                },
            })
            .await;

        match account_staked_balance_response {
            Ok(response) => {
                let call_result =
                    if let near_jsonrpc_primitives::types::query::QueryResponseKind::CallResult(
                        result,
                    ) = response.kind
                    {
                        result
                    } else {
                        bail!("Unexpected response: {:#?}", response);
                    };
                let token: String = serde_json::from_slice(&call_result.result)?;
                let token: u128 = token.parse()?;
                Ok(NearToken::from_yoctonear(token))
            }
            Err(near_jsonrpc_client::errors::JsonRpcError::ServerError(
                near_jsonrpc_client::errors::JsonRpcServerError::HandlerError(
                    near_jsonrpc_client::methods::query::RpcQueryError::NoContractCode { .. }
                    | near_jsonrpc_client::methods::query::RpcQueryError::ContractExecutionError {
                        ..
                    },
                ),
            )) => Ok(near_token::NearToken::from_yoctonear(0)),
            Err(err) => Err(err.into()),
        }
    }

    pub async fn delegations(&self) -> anyhow::Result<BTreeMap<AccountId, NearToken>> {
        let validators = if let Ok(fastnear) = self.client.fastnear() {
            fastnear.account_delegated_in(&self.account_id).await?
        } else if let Ok(staking) = self.client.stake() {
            staking.staking_pools().await?
        } else {
            bail!("FastNear and Staking pool factory are not set");
        };

        futures::stream::iter(validators)
            .map(|validator_account_id| async {
                let balance = self.delegation_in_pool(&validator_account_id).await?;
                Ok::<_, anyhow::Error>((validator_account_id, balance))
            })
            .buffer_unordered(self.client.concurrency_limit)
            .filter(|balance_result| {
                futures::future::ready(if let Ok((_, balance)) = balance_result {
                    !balance.is_zero()
                } else {
                    true
                })
            })
            .try_collect()
            .await
    }
}

#[cfg(test)]
mod tests {
    const TESTNET_ACCOUNT: &str = "yurtur.testnet";
    const MAINNET_ACCOUTN: &str = "yurturdev.near";

    use crate::{config::Config, Client};

    #[tokio::test]
    async fn load_account() {
        let config = Config::default();
        let client = Client::with_config(config.network_connection["testnet"].clone());
        let account = client.account(TESTNET_ACCOUNT.parse().unwrap());
        assert!(account.account().await.is_ok());
        assert!(account.list_keys().await.is_ok());
        assert!(account.delegations().await.is_ok());
    }

    #[tokio::test]
    async fn delegations_fastnear() {
        let config = Config::default();
        let mut config = config.network_connection["mainnet"].clone();
        assert!(config.fastnear_url.is_some());
        let client = Client::with_config(config.clone());

        let account = client.account(MAINNET_ACCOUTN.parse().unwrap());
        assert!(account.delegations().await.is_ok());

        config.fastnear_url = None;
        let client = Client::with_config(config);

        let account = client.account(MAINNET_ACCOUTN.parse().unwrap());
        assert!(account.delegations().await.is_ok());
    }
}
