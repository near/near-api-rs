use near_gas::NearGas;
use near_primitives::{
    action::{Action, DeployContractAction, FunctionCallAction},
    types::{AccountId, StoreKey},
    views::ViewStateResult,
};
use near_token::NearToken;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    query::{CallResultHandler, QueryBuilder, ViewCodeHandler, ViewStateHandler},
    transactions::{ConstructTransaction, Transaction},
};

pub struct Contract(pub AccountId);

impl Contract {
    pub fn view<Args, Response>(
        &self,
        method_name: &str,
        args: Args,
    ) -> anyhow::Result<QueryBuilder<CallResultHandler<Response, Response>>>
    where
        Args: serde::Serialize,
        Response: DeserializeOwned,
    {
        let args = serde_json::to_vec(&args)?;
        let request = near_primitives::views::QueryRequest::CallFunction {
            account_id: self.0.clone(),
            method_name: method_name.to_owned(),
            args: near_primitives::types::FunctionArgs::from(args),
        };

        Ok(QueryBuilder::new(request, CallResultHandler::default()))
    }

    pub fn transact<Args: Serialize>(
        &self,
        method_name: &str,
        args: Args,
    ) -> anyhow::Result<ContractTransactBuilder> {
        let args = serde_json::to_vec(&args)?;

        Ok(ContractTransactBuilder::call(
            self.0.clone(),
            method_name.to_owned(),
            args,
        ))
    }

    pub fn deploy(&self, code: Vec<u8>) -> ConstructTransaction {
        Transaction::construct(self.0.clone(), self.0.clone())
            .add_action(Action::DeployContract(DeployContractAction { code }))
    }

    pub fn deploy_with_init<Args: Serialize>(
        &self,
        code: Vec<u8>,
        method_name: &str,
        args: Args,
    ) -> anyhow::Result<ContractTransactBuilder> {
        let args = serde_json::to_vec(&args)?;

        Ok(ContractTransactBuilder::deploy_with_init(
            self.0.clone(),
            method_name.to_string(),
            args,
            code,
        ))
    }

    pub fn abi(&self) -> QueryBuilder<CallResultHandler<Vec<u8>, Option<near_abi::AbiRoot>>> {
        let request = near_primitives::views::QueryRequest::CallFunction {
            account_id: self.0.clone(),
            method_name: "__contract_abi".to_owned(),
            args: near_primitives::types::FunctionArgs::from(vec![]),
        };

        QueryBuilder::new(
            request,
            CallResultHandler::with_postprocess(|data: Vec<u8>| {
                serde_json::from_slice(zstd::decode_all(data.as_slice()).ok()?.as_slice()).ok()
            }),
        )
    }

    pub fn wasm(&self) -> QueryBuilder<ViewCodeHandler> {
        let request = near_primitives::views::QueryRequest::ViewCode {
            account_id: self.0.clone(),
        };

        QueryBuilder::new(request, ViewCodeHandler)
    }

    pub fn view_storage_with_prefix(
        &self,
        prefix: Vec<u8>,
    ) -> QueryBuilder<ViewStateHandler<ViewStateResult>> {
        let request = near_primitives::views::QueryRequest::ViewState {
            account_id: self.0.clone(),
            prefix: StoreKey::from(prefix),
            include_proof: false,
        };

        QueryBuilder::new(request, ViewStateHandler::default())
    }

    pub fn view_storage(&self) -> QueryBuilder<ViewStateHandler<ViewStateResult>> {
        self.view_storage_with_prefix(vec![])
    }
}

pub struct ContractTransactBuilder {
    contract: AccountId,
    method_name: String,
    code: Option<Vec<u8>>,
    args: Vec<u8>,
    gas: Option<NearGas>,
    deposit: Option<NearToken>,
}

impl ContractTransactBuilder {
    pub fn call(contract: AccountId, method_name: String, args: Vec<u8>) -> Self {
        Self {
            contract,
            method_name,
            args,
            code: None,
            gas: None,
            deposit: None,
        }
    }

    pub fn deploy_with_init(
        contract: AccountId,
        method_name: String,
        args: Vec<u8>,
        code: Vec<u8>,
    ) -> Self {
        Self {
            contract,
            method_name,
            code: Some(code),
            args,
            gas: None,
            deposit: None,
        }
    }

    pub fn gas(mut self, gas: NearGas) -> Self {
        self.gas = Some(gas);
        self
    }

    pub fn deposit(mut self, deposit: NearToken) -> Self {
        self.deposit = Some(deposit);
        self
    }

    pub fn construct_tx(self, signer_id: AccountId) -> ConstructTransaction {
        let gas = self.gas.unwrap_or_else(|| NearGas::from_tgas(100));
        let deposit = self.deposit.unwrap_or_else(|| NearToken::from_yoctonear(0));

        let tx: ConstructTransaction = if let Some(code) = self.code {
            Transaction::construct(signer_id, self.contract)
                .add_action(Action::DeployContract(DeployContractAction { code }))
        } else {
            Transaction::construct(signer_id.clone(), self.contract)
        };

        tx.add_action(Action::FunctionCall(Box::new(FunctionCallAction {
            method_name: self.method_name.to_owned(),
            args: self.args,
            gas: gas.as_gas(),
            deposit: deposit.as_yoctonear(),
        })))
    }
}

#[cfg(test)]
mod tests {
    use near_gas::NearGas;

    use crate::sign::Signer;

    #[derive(serde::Serialize)]
    pub struct Paging {
        limit: u32,
        page: u32,
    }

    #[tokio::test]
    async fn fetch_from_contract() {
        let result: serde_json::Value =
            crate::contract::Contract("race-of-sloths-stage.testnet".parse().unwrap())
                .view("prs", Paging { limit: 5, page: 1 })
                .unwrap()
                .fetch_from_testnet()
                .await
                .unwrap()
                .data;

        assert!(result.is_array());
    }

    #[tokio::test]
    async fn fetch_storage() {
        let result = crate::contract::Contract("race-of-sloths-stage.testnet".parse().unwrap())
            .view_storage()
            .fetch_from_testnet()
            .await
            .unwrap();

        println!("{:?}", result.data);
    }

    #[tokio::test]
    async fn exec_contract() {
        crate::contract::Contract("yurtur.testnet".parse().unwrap())
            .transact(
                "flip_coin",
                serde_json::json!({
                    "player_guess": "tails"
                }),
            )
            .unwrap()
            .gas(NearGas::from_tgas(100))
            .construct_tx("yurtur.testnet".parse().unwrap())
            .with_signer(Signer::seed_phrase(
                include_str!("../seed_phrase").to_string(),
            ))
            .send_to_testnet()
            .await
            .unwrap()
            .assert_success();
    }

    #[tokio::test]
    async fn deploy_contract() {
        crate::contract::Contract("yurtur.testnet".parse().unwrap())
            .deploy(include_bytes!("../contract_rs.wasm").to_vec())
            .with_signer(Signer::seed_phrase(
                include_str!("../seed_phrase").to_string(),
            ))
            .send_to_testnet()
            .await
            .unwrap()
            .assert_success();
    }
}
