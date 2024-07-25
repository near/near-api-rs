use std::marker::PhantomData;

use near_gas::NearGas;

use near_primitives::{
    action::{Action, DeployContractAction, FunctionCallAction},
    types::{AccountId, BlockReference, StoreKey},
};
use near_token::NearToken;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    common::{
        query::{
            CallResultHandler, PostprocessHandler, QueryBuilder, SimpleQuery, ViewCodeHandler,
            ViewStateHandler,
        },
        send::ExecuteSignedTransaction,
    },
    signer::Signer,
    transactions::{ConstructTransaction, Transaction},
    types::{contract::ContractSourceMetadata, Data},
};

pub struct Contract(pub AccountId);

impl Contract {
    pub fn call_function<Args>(
        self,
        method_name: &str,
        args: Args,
    ) -> anyhow::Result<CallFunctionBuilder>
    where
        Args: serde::Serialize,
    {
        let args = serde_json::to_vec(&args)?;

        Ok(CallFunctionBuilder {
            contract: self.0.clone(),
            method_name: method_name.to_string(),
            args,
        })
    }

    pub fn deploy(self, code: Vec<u8>) -> DeployContractBuilder {
        DeployContractBuilder::new(self.0.clone(), code)
    }

    pub fn abi(
        self,
    ) -> QueryBuilder<PostprocessHandler<Option<near_abi::AbiRoot>, CallResultHandler<Vec<u8>>>>
    {
        let request = near_primitives::views::QueryRequest::CallFunction {
            account_id: self.0.clone(),
            method_name: "__contract_abi".to_owned(),
            args: near_primitives::types::FunctionArgs::from(vec![]),
        };

        QueryBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            PostprocessHandler::new(
                CallResultHandler::default(),
                Box::new(|data: Data<Vec<u8>>| {
                    serde_json::from_slice(zstd::decode_all(data.data.as_slice()).ok()?.as_slice())
                        .ok()
                }),
            ),
        )
    }

    pub fn wasm(self) -> QueryBuilder<ViewCodeHandler> {
        let request = near_primitives::views::QueryRequest::ViewCode {
            account_id: self.0.clone(),
        };

        QueryBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            ViewCodeHandler,
        )
    }

    pub fn view_storage_with_prefix(self, prefix: Vec<u8>) -> QueryBuilder<ViewStateHandler> {
        let request = near_primitives::views::QueryRequest::ViewState {
            account_id: self.0.clone(),
            prefix: StoreKey::from(prefix),
            include_proof: false,
        };

        QueryBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            ViewStateHandler,
        )
    }

    pub fn view_storage(self) -> QueryBuilder<ViewStateHandler> {
        self.view_storage_with_prefix(vec![])
    }

    pub fn contract_source_metadata(
        self,
    ) -> QueryBuilder<CallResultHandler<ContractSourceMetadata>> {
        self.call_function("contract_source_metadata", ())
            .expect("arguments are always serializable")
            .read_only()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct DeployContractBuilder {
    contract: AccountId,
    code: Vec<u8>,
}

impl DeployContractBuilder {
    pub fn new(contract: AccountId, code: Vec<u8>) -> Self {
        Self { contract, code }
    }

    pub fn without_init_call(self) -> ConstructTransaction {
        Transaction::construct(self.contract.clone(), self.contract.clone()).add_action(
            Action::DeployContract(DeployContractAction { code: self.code }),
        )
    }

    pub fn with_init_call<Args: Serialize>(
        self,
        method_name: &str,
        args: Args,
    ) -> anyhow::Result<ConstructTransaction> {
        let args = serde_json::to_vec(&args)?;

        Ok(ContractTransactBuilder::new(
            self.contract.clone(),
            method_name.to_string(),
            args,
            Some(Action::DeployContract(DeployContractAction {
                code: self.code,
            })),
        )
        .with_signer_account(self.contract))
    }
}

pub struct CallFunctionBuilder {
    contract: AccountId,
    method_name: String,
    args: Vec<u8>,
}

impl CallFunctionBuilder {
    pub fn read_only<Response: DeserializeOwned>(
        self,
    ) -> QueryBuilder<CallResultHandler<Response>> {
        let request = near_primitives::views::QueryRequest::CallFunction {
            account_id: self.contract,
            method_name: self.method_name,
            args: near_primitives::types::FunctionArgs::from(self.args),
        };

        QueryBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            CallResultHandler(PhantomData),
        )
    }

    pub fn transaction(self) -> ContractTransactBuilder {
        ContractTransactBuilder::new(self.contract, self.method_name, self.args, None)
    }
}

pub struct ContractTransactBuilder {
    contract: AccountId,
    method_name: String,
    args: Vec<u8>,
    pre_action: Option<Action>,
    gas: Option<NearGas>,
    deposit: Option<NearToken>,
}

impl ContractTransactBuilder {
    fn new(
        contract: AccountId,
        method_name: String,
        args: Vec<u8>,
        pre_action: Option<Action>,
    ) -> Self {
        Self {
            contract,
            method_name,
            args,
            pre_action,
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

    pub fn with_signer(
        self,
        signer_id: AccountId,
        signer: Signer,
    ) -> ExecuteSignedTransaction<ConstructTransaction> {
        self.with_signer_account(signer_id).with_signer(signer)
    }

    // Re-used by stake.rs as we do have extra signer_id context, but we don't need there a signer
    pub(crate) fn with_signer_account(self, signer_id: AccountId) -> ConstructTransaction {
        let gas = self.gas.unwrap_or_else(|| NearGas::from_tgas(100));
        let deposit = self.deposit.unwrap_or_else(|| NearToken::from_yoctonear(0));

        let tx: ConstructTransaction = if let Some(preaction) = self.pre_action {
            Transaction::construct(signer_id, self.contract).add_action(preaction)
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

    use crate::signer::Signer;

    #[derive(serde::Serialize)]
    pub struct Paging {
        limit: u32,
        page: u32,
    }

    #[tokio::test]
    async fn fetch_from_contract() {
        let result: serde_json::Value =
            crate::contract::Contract("race-of-sloths-stage.testnet".parse().unwrap())
                .call_function("prs", Paging { limit: 5, page: 1 })
                .unwrap()
                .read_only()
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
            .call_function(
                "flip_coin",
                serde_json::json!({
                    "player_guess": "tails"
                }),
            )
            .unwrap()
            .transaction()
            .gas(NearGas::from_tgas(100))
            .with_signer(
                "yurtur.testnet".parse().unwrap(),
                Signer::seed_phrase(include_str!("../seed_phrase").to_string(), None).unwrap(),
            )
            .send_to_testnet()
            .await
            .unwrap()
            .assert_success();
    }

    #[tokio::test]
    async fn deploy_contract() {
        crate::contract::Contract("yurtur.testnet".parse().unwrap())
            .deploy(include_bytes!("../contract_rs.wasm").to_vec())
            .without_init_call()
            .with_signer(
                Signer::seed_phrase(include_str!("../seed_phrase").to_string(), None).unwrap(),
            )
            .send_to_testnet()
            .await
            .unwrap()
            .assert_success();
    }
}
