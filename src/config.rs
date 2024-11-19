#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkConfig {
    pub network_name: String,
    pub rpc_url: url::Url,
    pub rpc_api_key: Option<crate::types::ApiKey>,
    // https://github.com/near/near-cli-rs/issues/116
    pub linkdrop_account_id: Option<near_primitives::types::AccountId>,
    // https://docs.near.org/social/contract
    pub near_social_db_contract_account_id: Option<near_primitives::types::AccountId>,
    pub faucet_url: Option<url::Url>,
    pub meta_transaction_relayer_url: Option<url::Url>,
    pub fastnear_url: Option<url::Url>,
    pub staking_pools_factory_account_id: Option<near_primitives::types::AccountId>,
}

impl NetworkConfig {
    pub fn mainnet() -> Self {
        Self {
            network_name: "mainnet".to_string(),
            rpc_url: "https://archival-rpc.mainnet.near.org".parse().unwrap(),
            rpc_api_key: None,
            linkdrop_account_id: Some("near".parse().unwrap()),
            near_social_db_contract_account_id: Some("social.near".parse().unwrap()),
            faucet_url: None,
            meta_transaction_relayer_url: None,
            fastnear_url: Some("https://api.fastnear.com/".parse().unwrap()),
            staking_pools_factory_account_id: Some("pool.near".parse().unwrap()),
        }
    }

    pub fn testnet() -> Self {
        Self {
            network_name: "testnet".to_string(),
            rpc_url: "https://archival-rpc.testnet.near.org".parse().unwrap(),
            rpc_api_key: None,
            linkdrop_account_id: Some("testnet".parse().unwrap()),
            near_social_db_contract_account_id: Some("v1.social08.testnet".parse().unwrap()),
            faucet_url: Some("https://helper.nearprotocol.com/account".parse().unwrap()),
            meta_transaction_relayer_url: Some("http://localhost:3030/relay".parse().unwrap()),
            fastnear_url: None,
            staking_pools_factory_account_id: Some("pool.f863973.m0".parse().unwrap()),
        }
    }

    pub fn json_rpc_client(&self) -> near_jsonrpc_client::JsonRpcClient {
        let mut json_rpc_client =
            near_jsonrpc_client::JsonRpcClient::connect(self.rpc_url.as_ref());
        if let Some(rpc_api_key) = &self.rpc_api_key {
            json_rpc_client =
                json_rpc_client.header(near_jsonrpc_client::auth::ApiKey::from(rpc_api_key.clone()))
        };
        json_rpc_client
    }
}

#[cfg(feature = "workspaces")]
impl<T: near_workspaces::Network> From<near_workspaces::Worker<T>> for NetworkConfig {
    fn from(network: near_workspaces::Worker<T>) -> Self {
        use near_workspaces::network::NetworkInfo;

        let info = network.info();
        Self {
            network_name: info.name.clone(),
            rpc_url: info.rpc_url.clone(),
            rpc_api_key: None,
            linkdrop_account_id: None,
            near_social_db_contract_account_id: None,
            faucet_url: None,
            meta_transaction_relayer_url: None,
            fastnear_url: None,
            staking_pools_factory_account_id: None,
        }
    }
}
