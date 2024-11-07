use std::{marker::PhantomData, sync::Arc};

use near_gas::NearGas;

use near_primitives::{
    action::{Action, DeployContractAction, FunctionCallAction},
    types::BlockReference,
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
    errors::BuilderError,
    signer::Signer,
    transactions::{ConstructTransaction, Transaction},
    types::{contract::ContractSourceMetadata, views::StoreKey, AccountId, Data},
};

#[derive(Clone, Debug)]
pub struct Contract(pub AccountId);

impl Contract {
    pub fn call_function<Args>(
        &self,
        method_name: &str,
        args: Args,
    ) -> Result<CallFunctionBuilder, BuilderError>
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

    pub fn deploy(&self, code: Vec<u8>) -> DeployContractBuilder {
        DeployContractBuilder::new(self.0.clone(), code)
    }

    pub fn abi(
        &self,
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

    pub fn wasm(&self) -> QueryBuilder<ViewCodeHandler> {
        let request = near_primitives::views::QueryRequest::ViewCode {
            account_id: self.0.clone(),
        };

        QueryBuilder::new(
            SimpleQuery { request },
            BlockReference::latest(),
            ViewCodeHandler,
        )
    }

    pub fn view_storage_with_prefix(&self, prefix: Vec<u8>) -> QueryBuilder<ViewStateHandler> {
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

    pub fn view_storage(&self) -> QueryBuilder<ViewStateHandler> {
        self.view_storage_with_prefix(vec![])
    }

    pub fn contract_source_metadata(
        &self,
    ) -> QueryBuilder<CallResultHandler<ContractSourceMetadata>> {
        self.call_function("contract_source_metadata", ())
            .expect("arguments are always serializable")
            .read_only()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct DeployContractBuilder {
    contract: AccountId,
    code: Vec<u8>,
}

impl DeployContractBuilder {
    pub const fn new(contract: AccountId, code: Vec<u8>) -> Self {
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
    ) -> Result<ConstructTransaction, BuilderError> {
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

#[derive(Clone, Debug)]
pub struct CallFunctionBuilder {
    contract: AccountId,
    method_name: String,
    args: Vec<u8>,
}

impl CallFunctionBuilder {
    pub fn read_only<Response: Send + Sync + DeserializeOwned>(
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

#[derive(Clone, Debug)]
pub struct ContractTransactBuilder {
    contract: AccountId,
    method_name: String,
    args: Vec<u8>,
    pre_action: Option<Action>,
    gas: Option<NearGas>,
    deposit: Option<NearToken>,
}

impl ContractTransactBuilder {
    const fn new(
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

    pub const fn gas(mut self, gas: NearGas) -> Self {
        self.gas = Some(gas);
        self
    }

    pub const fn deposit(mut self, deposit: NearToken) -> Self {
        self.deposit = Some(deposit);
        self
    }

    pub fn with_signer(
        self,
        signer_id: AccountId,
        signer: Arc<Signer>,
    ) -> ExecuteSignedTransaction {
        self.with_signer_account(signer_id).with_signer(signer)
    }

    // Re-used by stake.rs and tokens.rs as we do have extra signer_id context, but we don't need there a signer
    pub(crate) fn with_signer_account(self, signer_id: AccountId) -> ConstructTransaction {
        let gas = self.gas.unwrap_or_else(|| NearGas::from_tgas(100));
        let deposit = self.deposit.unwrap_or_else(|| NearToken::from_yoctonear(0));

        let tx: ConstructTransaction = if let Some(preaction) = self.pre_action {
            Transaction::construct(signer_id, self.contract).add_action(preaction)
        } else {
            Transaction::construct(signer_id, self.contract)
        };

        tx.add_action(Action::FunctionCall(Box::new(FunctionCallAction {
            method_name: self.method_name.to_owned(),
            args: self.args,
            gas: gas.as_gas(),
            deposit: deposit.as_yoctonear(),
        })))
    }
}
