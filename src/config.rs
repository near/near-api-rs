use near_jsonrpc_client::JsonRpcClient;

use crate::errors::RetryError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// Using this struct to configure RPC endpoints.
/// This is primary way to configure retry logic.
pub struct RPCEndpoint {
    pub url: url::Url,
    pub api_key: Option<crate::types::ApiKey>,
    /// Number of consecutive failures to move on to the next endpoint.
    pub retries: u8,
    pub exponential_backoff: bool,
    pub factor: u8,
    pub initial_sleep: std::time::Duration,
}

impl RPCEndpoint {
    pub const fn new(url: url::Url) -> Self {
        Self {
            url,
            api_key: None,
            retries: 5,
            exponential_backoff: true,
            factor: 2,
            // 10ms, 20ms, 40ms, 80ms, 160ms
            initial_sleep: std::time::Duration::from_millis(10),
        }
    }

    pub fn mainnet() -> Self {
        Self::new("https://archival-rpc.mainnet.near.org".parse().unwrap())
    }

    pub fn testnet() -> Self {
        Self::new("https://archival-rpc.testnet.near.org".parse().unwrap())
    }

    /// Set API key for the endpoint.
    pub fn with_api_key(mut self, api_key: crate::types::ApiKey) -> Self {
        self.api_key = Some(api_key);
        self
    }

    /// Set number of retries for the endpoint before moving on to the next one.
    pub const fn with_retries(mut self, retries: u8) -> Self {
        self.retries = retries;
        self
    }

    /// Should we use exponential backoff for the endpoint. Default is true.
    pub const fn with_exponential_backoff(mut self, exponential_backoff: bool, factor: u8) -> Self {
        self.exponential_backoff = exponential_backoff;
        self.factor = factor;
        self
    }

    /// Set initial sleep duration for the endpoint. Default is 10ms.
    pub const fn with_initial_sleep(mut self, initial_sleep: std::time::Duration) -> Self {
        self.initial_sleep = initial_sleep;
        self
    }

    pub fn get_sleep_duration(&self, retry: usize) -> std::time::Duration {
        if self.exponential_backoff {
            self.initial_sleep * ((self.factor as u32).pow(retry as u32))
        } else {
            self.initial_sleep
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkConfig {
    pub network_name: String,
    pub rpc_endpoints: Vec<RPCEndpoint>,
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
            rpc_endpoints: vec![RPCEndpoint::mainnet()],
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
            rpc_endpoints: vec![RPCEndpoint::testnet()],
            linkdrop_account_id: Some("testnet".parse().unwrap()),
            near_social_db_contract_account_id: Some("v1.social08.testnet".parse().unwrap()),
            faucet_url: Some("https://helper.nearprotocol.com/account".parse().unwrap()),
            meta_transaction_relayer_url: Some("http://localhost:3030/relay".parse().unwrap()),
            fastnear_url: None,
            staking_pools_factory_account_id: Some("pool.f863973.m0".parse().unwrap()),
        }
    }

    pub(crate) fn json_rpc_client(&self, index: usize) -> near_jsonrpc_client::JsonRpcClient {
        let rpc_endpoint = &self.rpc_endpoints[index];
        let mut json_rpc_client =
            near_jsonrpc_client::JsonRpcClient::connect(rpc_endpoint.url.as_ref());
        if let Some(rpc_api_key) = &rpc_endpoint.api_key {
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
            rpc_endpoints: vec![RPCEndpoint::new(info.rpc_url.clone())],
            linkdrop_account_id: None,
            near_social_db_contract_account_id: None,
            faucet_url: None,
            meta_transaction_relayer_url: None,
            fastnear_url: None,
            staking_pools_factory_account_id: None,
        }
    }
}

#[derive(Debug)]
pub enum RetryResponse<R, E> {
    Ok(R),
    Retry(E),
    Critical(E),
}

impl<R, E> From<Result<R, E>> for RetryResponse<R, E> {
    fn from(value: Result<R, E>) -> Self {
        match value {
            Ok(value) => Self::Ok(value),
            Err(value) => Self::Retry(value),
        }
    }
}

pub async fn retry<R, E, T, F>(network: NetworkConfig, mut task: F) -> Result<R, RetryError<E>>
where
    F: FnMut(JsonRpcClient) -> T + Send,
    T: core::future::Future<Output = RetryResponse<R, E>> + Send,
    T::Output: Send,
    E: Send,
{
    if network.rpc_endpoints.is_empty() {
        return Err(RetryError::NoRpcEndpoints);
    }

    let mut last_error = None;
    for (index, endpoint) in network.rpc_endpoints.iter().enumerate() {
        let client = network.json_rpc_client(index);
        for retry in 0..endpoint.retries {
            let result = task(client.clone()).await;
            match result {
                RetryResponse::Ok(result) => return Ok(result),
                RetryResponse::Retry(error) => {
                    last_error = Some(error);
                    tokio::time::sleep(endpoint.get_sleep_duration(retry as usize)).await;
                }
                RetryResponse::Critical(result) => return Err(RetryError::Critical(result)),
            }
        }
    }
    Err(RetryError::RetriesExhausted(last_error.expect(
        "Logic error: last_error should be Some when all retries are exhausted",
    )))
}
