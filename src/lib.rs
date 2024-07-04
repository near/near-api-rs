use config::NetworkConfig;
use near_primitives::types::AccountId;

pub mod account;
pub mod config;
pub mod contract;
pub mod fastnear;
pub mod stake;
pub mod types;

pub struct Client {
    config: NetworkConfig,
    json_rpc_client: near_jsonrpc_client::JsonRpcClient,
    concurrency_limit: usize,
}

impl Client {
    pub fn with_config(config: NetworkConfig) -> Self {
        let json_rpc_client = config.json_rpc_client();
        Self {
            config,
            json_rpc_client,
            concurrency_limit: 10,
        }
    }

    pub fn account(&self, account_id: AccountId) -> account::AccountHandler {
        account::AccountHandler::new(self, account_id)
    }

    pub fn fastnear(&self) -> anyhow::Result<fastnear::FastNearHandler> {
        fastnear::FastNearHandler::new(self)
    }

    pub fn stake(&self) -> anyhow::Result<stake::StakingHandler> {
        stake::StakingHandler::new(self)
    }

    pub fn contract(&self, contract_id: AccountId) -> contract::ContractHandler {
        contract::ContractHandler::new(self, contract_id)
    }
}
