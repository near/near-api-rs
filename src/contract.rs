use std::{marker::PhantomData, sync::Arc};

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
    errors::BuilderError,
    signer::Signer,
    transactions::{ConstructTransaction, Transaction},
    types::{contract::ContractSourceMetadata, Data},
};

/// Contract-related interactions with the NEAR Protocol
///
/// The [`Contract`] struct provides methods to interact with NEAR contracts, including calling functions, querying storage, and deploying contracts.
///
/// # Examples
///
/// ```rust,no_run
/// use near_api::*;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let abi = Contract("some_contract.testnet".parse()?).abi().fetch_from_testnet().await?;
/// println!("ABI: {:?}", abi);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct Contract(pub AccountId);

impl Contract {
    /// Prepares a call to a contract function.
    ///
    /// This will return a builder that can be used to prepare a query or a transaction.
    ///
    /// ## Calling view function `get_number`
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let number: Data<u64> = Contract("some_contract.testnet".parse()?)
    ///     .call_function("get_number", ())?
    ///     .read_only()
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Number: {:?}", number);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Calling a state changing function `set_number`
    /// ```rust,no_run
    /// use near_api::*;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// let result: near_primitives::views::FinalExecutionOutcomeView = Contract("some_contract.testnet".parse()?)
    ///     .call_function("set_number", json!({ "number": 100 }))?
    ///     .transaction()
    ///      // Optional
    ///     .gas(NearGas::from_tgas(200))
    ///     .with_signer("alice.testnet".parse()?, signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
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

    /// Prepares a transaction to deploy a contract to the provided account.
    ///
    /// The code is the wasm bytecode of the contract. For more information on how to compile your contract,
    /// please refer to the [NEAR documentation](https://docs.near.org/build/smart-contracts/quickstart).
    ///
    /// ## Deploying the contract
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = std::fs::read("path/to/your/contract.wasm")?;
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// let result: near_primitives::views::FinalExecutionOutcomeView = Contract::deploy("contract.testnet".parse()?, code)
    ///     .without_init_call()
    ///     .with_signer(signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Deploying the contract with an init call
    /// ```rust,no_run
    /// use near_api::*;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = std::fs::read("path/to/your/contract.wasm")?;
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// let result: near_primitives::views::FinalExecutionOutcomeView = Contract::deploy("contract.testnet".parse()?, code)
    ///     .with_init_call("init", json!({ "number": 100 }))?
    ///     // Optional
    ///     .gas(NearGas::from_tgas(200))
    ///     .with_signer(signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub const fn deploy(contract: AccountId, code: Vec<u8>) -> DeployContractBuilder {
        DeployContractBuilder::new(contract, code)
    }

    /// Prepares a query to fetch the [ABI](near_abi::AbiRoot) of the contract using the following [standard](https://github.com/near/near-abi-rs).
    ///
    /// Please be aware that not all the contracts provide the ABI.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let abi = Contract("some_contract.testnet".parse()?).abi().fetch_from_testnet().await?;
    /// println!("ABI: {:?}", abi);
    /// # Ok(())
    /// # }
    /// ```
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

    /// Prepares a query to fetch the wasm code ([Data]<[ContractCodeView](near_primitives::views::ContractCodeView)>) of the contract.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let wasm = Contract("some_contract.testnet".parse()?).wasm().fetch_from_testnet().await?;
    /// println!("WASM: {:?}", wasm.data.code.len());
    /// # Ok(())
    /// # }
    /// ```
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

    /// Prepares a query to fetch the storage of the contract ([Data]<[ViewStateResult](near_primitives::views::ViewStateResult)>) using the given prefix as a filter.
    ///
    /// It helpful if you are aware of the storage that you are looking for.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = Contract("some_contract.testnet".parse()?)
    ///     .view_storage_with_prefix(b"se".to_vec())
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Storage: {:?}", storage);
    /// # Ok(())
    /// # }
    /// ```
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

    /// Prepares a query to fetch the storage of the contract ([Data]<[ViewStateResult](near_primitives::views::ViewStateResult)>).
    ///
    /// Please be aware that large storage queries might fail.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = Contract("some_contract.testnet".parse()?)
    ///     .view_storage()
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Storage: {:?}", storage);
    /// # Ok(())
    /// # }
    /// ```
    pub fn view_storage(&self) -> QueryBuilder<ViewStateHandler> {
        self.view_storage_with_prefix(vec![])
    }

    /// Prepares a query to fetch the contract source metadata([Data]<[ContractSourceMetadata]>) using [NEP-330](https://github.com/near/NEPs/blob/master/neps/nep-0330.md) standard.
    ///
    /// The contract source metadata is a standard interface that allows auditing and viewing source code for a deployed smart contract.
    /// Implementation of this standard is purely optional but is recommended for developers whose contracts are open source.
    ///
    /// # Examples 
    /// 
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let source_metadata = Contract("some_contract.testnet".parse()?)
    ///     .contract_source_metadata()
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Source metadata: {:?}", source_metadata);
    /// # Ok(())
    /// # }
    /// ```
    /// A more verbose runnable example is present in `examples/contract_source_metadata.rs`:
    /// ```rust,no_run
    #[doc = include_str!("../examples/contract_source_metadata.rs")]
    /// ```
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

    /// Prepares a transaction to deploy a contract to the provided account without an init call.
    ///
    /// This will deploy the contract without calling any function.
    pub fn without_init_call(self) -> ConstructTransaction {
        Transaction::construct(self.contract.clone(), self.contract.clone()).add_action(
            Action::DeployContract(DeployContractAction { code: self.code }),
        )
    }

    /// Prepares a transaction to deploy a contract to the provided account with an init call.
    ///
    /// This will deploy the contract and call the init function with the provided arguments as a single transaction.
    pub fn with_init_call<Args: Serialize>(
        self,
        method_name: &str,
        args: Args,
    ) -> Result<DeployContractTransactBuilder, BuilderError> {
        let args = serde_json::to_vec(&args)?;

        Ok(DeployContractTransactBuilder::new(
            self.contract.clone(),
            method_name.to_string(),
            args,
            self.code,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct DeployContractTransactBuilder {
    contract: AccountId,
    method_name: String,
    args: Vec<u8>,
    code: Vec<u8>,
    gas: Option<NearGas>,
    deposit: Option<NearToken>,
}

impl DeployContractTransactBuilder {
    const fn new(contract: AccountId, method_name: String, args: Vec<u8>, code: Vec<u8>) -> Self {
        Self {
            contract,
            method_name,
            args,
            code,
            gas: None,
            deposit: None,
        }
    }

    /// Specify the gas limit for the transaction. By default it is set to 100 TGas.
    pub const fn gas(mut self, gas: NearGas) -> Self {
        self.gas = Some(gas);
        self
    }

    /// Specify the near deposit for the transaction. By default it is set to 0.
    ///
    /// Please note that the method should be [`payable`](https://docs.near.org/build/smart-contracts/anatomy/functions#payable-functions) in the contract to accept the deposit.
    /// Otherwise the transaction will fail.
    pub const fn deposit(mut self, deposit: NearToken) -> Self {
        self.deposit = Some(deposit);
        self
    }

    /// Specify the signer for the transaction. This will wrap-up the process of the preparing transaction.
    ///
    /// This will return the [`ExecuteSignedTransaction`] that can be used to sign and send the transaction to the network.
    pub fn with_signer(self, signer: Arc<Signer>) -> ExecuteSignedTransaction {
        let gas = self.gas.unwrap_or_else(|| NearGas::from_tgas(100));
        let deposit = self.deposit.unwrap_or_else(|| NearToken::from_yoctonear(0));

        Transaction::construct(self.contract.clone(), self.contract)
            .add_action(Action::DeployContract(DeployContractAction {
                code: self.code,
            }))
            .add_action(Action::FunctionCall(Box::new(FunctionCallAction {
                method_name: self.method_name.to_owned(),
                args: self.args,
                gas: gas.as_gas(),
                deposit: deposit.as_yoctonear(),
            })))
            .with_signer(signer)
    }
}

#[derive(Clone, Debug)]
pub struct CallFunctionBuilder {
    contract: AccountId,
    method_name: String,
    args: Vec<u8>,
}

impl CallFunctionBuilder {
    /// Prepares a read-only query that doesn't require a signing transaction.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let balance: Data<u64> = Contract("some_contract.testnet".parse()?).call_function("get_balance", ())?.read_only().fetch_from_testnet().await?;
    /// println!("Balance: {:?}", balance);
    ///
    /// let balance_at_block: Data<u64> = Contract("some_contract.testnet".parse()?).call_function("get_balance", ())?.read_only().at(Reference::AtBlock(1000000)).fetch_from_testnet().await?;
    /// println!("Balance at block 1000000: {:?}", balance_at_block);
    /// # Ok(())
    /// # }
    /// ```
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

    /// Prepares a transaction that will call a contract function leading to a state change.
    ///
    /// This will require a signer to be provided and gas to be paid.
    pub fn transaction(self) -> ContractTransactBuilder {
        ContractTransactBuilder::new(self.contract, self.method_name, self.args)
    }
}

#[derive(Clone, Debug)]
pub struct ContractTransactBuilder {
    contract: AccountId,
    method_name: String,
    args: Vec<u8>,
    gas: Option<NearGas>,
    deposit: Option<NearToken>,
}

impl ContractTransactBuilder {
    const fn new(contract: AccountId, method_name: String, args: Vec<u8>) -> Self {
        Self {
            contract,
            method_name,
            args,
            gas: None,
            deposit: None,
        }
    }

    /// Specify the gas limit for the transaction. By default it is set to 100 TGas.
    pub const fn gas(mut self, gas: NearGas) -> Self {
        self.gas = Some(gas);
        self
    }

    /// Specify the near deposit for the transaction. By default it is set to 0.
    ///
    /// Please note that the method should be [`payable`](https://docs.near.org/build/smart-contracts/anatomy/functions#payable-functions) in the contract to accept the deposit.
    /// Otherwise the transaction will fail.
    pub const fn deposit(mut self, deposit: NearToken) -> Self {
        self.deposit = Some(deposit);
        self
    }

    /// Specify the signer for the transaction. This will wrap-up the process of the preparing transaction.
    ///
    /// This will return the [`ExecuteSignedTransaction`] that can be used to sign and send the transaction to the network.
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

        Transaction::construct(signer_id, self.contract).add_action(Action::FunctionCall(Box::new(
            FunctionCallAction {
                method_name: self.method_name.to_owned(),
                args: self.args,
                gas: gas.as_gas(),
                deposit: deposit.as_yoctonear(),
            },
        )))
    }
}
