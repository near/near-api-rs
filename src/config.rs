use anyhow::Context;
use std::str::FromStr;
use tracing_indicatif::span_ext::IndicatifSpanExt;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkConfig {
    pub network_name: String,
    pub rpc_url: url::Url,
    pub rpc_api_key: Option<crate::types::ApiKey>,
    pub wallet_url: url::Url,
    pub explorer_transaction_url: url::Url,
    // https://github.com/near/near-cli-rs/issues/116
    pub linkdrop_account_id: Option<near_primitives::types::AccountId>,
    // https://docs.near.org/social/contract
    pub near_social_db_contract_account_id: Option<near_primitives::types::AccountId>,
    pub faucet_url: Option<url::Url>,
    pub meta_transaction_relayer_url: Option<url::Url>,
    pub fastnear_url: Option<url::Url>,
    pub staking_pools_factory_account_id: Option<near_primitives::types::AccountId>,
    pub coingecko_url: Option<url::Url>,
}

impl NetworkConfig {
    pub fn mainnet() -> Self {
        Self {
            network_name: "mainnet".to_string(),
            rpc_url: "https://archival-rpc.mainnet.near.org".parse().unwrap(),
            wallet_url: "https://app.mynearwallet.com/".parse().unwrap(),
            explorer_transaction_url: "https://explorer.near.org/transactions/".parse().unwrap(),
            rpc_api_key: None,
            linkdrop_account_id: Some("near".parse().unwrap()),
            near_social_db_contract_account_id: Some("social.near".parse().unwrap()),
            faucet_url: None,
            meta_transaction_relayer_url: None,
            fastnear_url: Some("https://api.fastnear.com/".parse().unwrap()),
            staking_pools_factory_account_id: Some("poolv1.near".parse().unwrap()),
            coingecko_url: Some("https://api.coingecko.com/".parse().unwrap()),
        }
    }

    pub fn testnet() -> Self {
        NetworkConfig {
            network_name: "testnet".to_string(),
            rpc_url: "https://archival-rpc.testnet.near.org".parse().unwrap(),
            wallet_url: "https://testnet.mynearwallet.com/".parse().unwrap(),
            explorer_transaction_url: "https://explorer.testnet.near.org/transactions/"
                .parse()
                .unwrap(),
            rpc_api_key: None,
            linkdrop_account_id: Some("testnet".parse().unwrap()),
            near_social_db_contract_account_id: Some("v1.social08.testnet".parse().unwrap()),
            faucet_url: Some("https://helper.nearprotocol.com/account".parse().unwrap()),
            meta_transaction_relayer_url: Some(
                "https://near-testnet.api.pagoda.co/relay".parse().unwrap(),
            ),
            fastnear_url: None,
            staking_pools_factory_account_id: Some("pool.f863973.m0".parse().unwrap()),
            coingecko_url: None,
        }
    }

    #[tracing::instrument(name = "Connecting to RPC", skip_all)]
    pub fn json_rpc_client(&self) -> near_jsonrpc_client::JsonRpcClient {
        tracing::Span::current().pb_set_message(self.rpc_url.as_str());
        let mut json_rpc_client =
            near_jsonrpc_client::JsonRpcClient::connect(self.rpc_url.as_ref());
        if let Some(rpc_api_key) = &self.rpc_api_key {
            json_rpc_client =
                json_rpc_client.header(near_jsonrpc_client::auth::ApiKey::from(rpc_api_key.clone()))
        };
        json_rpc_client
    }

    pub fn get_near_social_account_id_from_network(
        &self,
    ) -> anyhow::Result<near_primitives::types::AccountId> {
        if let Some(account_id) = self.near_social_db_contract_account_id.clone() {
            return Ok(account_id);
        }
        match self.network_name.as_str() {
            "mainnet" => {
                near_primitives::types::AccountId::from_str("social.near").context("Internal error")
            }
            "testnet" => near_primitives::types::AccountId::from_str("v1.social08.testnet")
                .context("Internal error"),
            _ => anyhow::bail!("This network does not provide the \"near-social\" contract"),
        }
    }
}
