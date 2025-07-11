use std::sync::Arc;

use near_types::{
    AccountId, Action, CryptoHash, Data, FunctionArgs, NearGas, NearToken, Reference, StoreKey,
    actions::{
        DeployContractAction, DeployGlobalContractAction, FunctionCallAction,
        GlobalContractDeployMode, GlobalContractIdentifier, UseGlobalContractAction,
    },
    contract::ContractSourceMetadata,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    advanced::{query_request::QueryRequest, query_rpc::SimpleQueryRpc},
    common::{
        query::{
            CallResultHandler, PostprocessHandler, QueryBuilder, ViewCodeHandler, ViewStateHandler,
        },
        send::ExecuteSignedTransaction,
        utils::to_base64,
    },
    errors::BuilderError,
    signer::Signer,
    transactions::{ConstructTransaction, SelfActionBuilder, Transaction},
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
    /// let result = Contract("some_contract.testnet".parse()?)
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
    /// let result = Contract::deploy("contract.testnet".parse()?)
    ///     .use_code(code)
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
    /// let result = Contract::deploy("contract.testnet".parse()?)
    ///     .use_code(code)
    ///     .with_init_call("init", json!({ "number": 100 }))?
    ///     // Optional
    ///     .gas(NearGas::from_tgas(200))
    ///     .with_signer(signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub const fn deploy(contract: AccountId) -> DeployBuilder {
        DeployBuilder::new(contract)
    }

    /// Prepares a transaction to deploy a code to the global contract code storage.
    ///
    /// This will allow other users to reference given code as hash or account-id and reduce
    /// the gas cost for deployment.
    ///
    /// Please be aware that the deploy costs 10x more compared to the regular costs and the tokens are burnt
    /// with no way to get it back.
    ///
    /// ## Example deploying a contract to the global contract code storage as hash
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = std::fs::read("path/to/your/contract.wasm")?;
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// let result = Contract::deploy_global_contract_code(code)
    ///     .as_hash()
    ///     .with_signer("some-account.testnet".parse()?, signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Example deploying a contract to the global contract code storage as account-id
    ///
    /// The difference between the hash and account-id version is that the account-id version
    /// upgradable and can be changed.
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = std::fs::read("path/to/your/contract.wasm")?;
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// let result = Contract::deploy_global_contract_code(code)
    ///     .as_account_id("nft-contract.testnet".parse()?)
    ///     .with_signer(signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub const fn deploy_global_contract_code(code: Vec<u8>) -> GlobalDeployBuilder {
        GlobalDeployBuilder::new(code)
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
    ) -> QueryBuilder<
        PostprocessHandler<Option<near_types::abi::AbiRoot>, CallResultHandler<Vec<u8>>>,
    > {
        self.call_function("__contract_abi", ())
            .expect("arguments are always serializable")
            .read_only()
            .map(|data: Data<Vec<u8>>| {
                serde_json::from_slice(zstd::decode_all(data.data.as_slice()).ok()?.as_slice()).ok()
            })
    }

    /// Prepares a query to fetch the wasm code ([Data]<[ContractCodeView](near_types::ContractCodeView)>) of the contract.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let wasm = Contract("some_contract.testnet".parse()?).wasm().fetch_from_testnet().await?;
    /// println!("WASM: {}", wasm.data.code_base64);
    /// # Ok(())
    /// # }
    /// ```
    pub fn wasm(&self) -> QueryBuilder<ViewCodeHandler> {
        let request = QueryRequest::ViewCode {
            account_id: self.0.clone(),
        };

        QueryBuilder::new(
            SimpleQueryRpc { request },
            Reference::Optimistic,
            ViewCodeHandler,
        )
    }

    /// Prepares a query to fetch the storage of the contract ([Data]<[ViewStateResult](near_types::ViewStateResult)>) using the given prefix as a filter.
    ///
    /// It helpful if you are aware of the storage that you are looking for.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = Contract("some_contract.testnet".parse()?)
    ///     .view_storage_with_prefix(b"se")
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Storage: {:?}", storage);
    /// # Ok(())
    /// # }
    /// ```
    pub fn view_storage_with_prefix(&self, prefix: &[u8]) -> QueryBuilder<ViewStateHandler> {
        let request = QueryRequest::ViewState {
            account_id: self.0.clone(),
            prefix_base64: StoreKey(to_base64(prefix)),
            include_proof: Some(false),
        };

        QueryBuilder::new(
            SimpleQueryRpc { request },
            Reference::Optimistic,
            ViewStateHandler,
        )
    }

    /// Prepares a query to fetch the storage of the contract ([Data]<[ViewStateResult](near_types::ViewStateResult)>).
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
        self.view_storage_with_prefix(&[])
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

#[derive(Clone, Debug)]
pub struct DeployBuilder {
    pub contract: AccountId,
}

impl DeployBuilder {
    pub const fn new(contract: AccountId) -> Self {
        Self { contract }
    }

    /// Prepares a transaction to deploy a contract to the provided account
    ///
    /// The code is the wasm bytecode of the contract. For more information on how to compile your contract,
    /// please refer to the [NEAR documentation](https://docs.near.org/build/smart-contracts/quickstart).
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = std::fs::read("path/to/your/contract.wasm")?;
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// let result = Contract::deploy("contract.testnet".parse()?)
    ///     .use_code(code)
    ///     .without_init_call()
    ///     .with_signer(signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    pub fn use_code(self, code: Vec<u8>) -> SetDeployActionBuilder {
        SetDeployActionBuilder::new(
            self.contract,
            Action::DeployContract(DeployContractAction { code }),
        )
    }

    // /// Prepares a transaction to deploy a contract to the provided account using a immutable hash reference to the code from the global contract code storage.
    // ///
    // /// # Example
    // /// ```rust,no_run
    // /// use near_api::*;
    // ///
    // /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    // /// let signer = Signer::new(Signer::from_ledger())?;
    // /// let result = Contract::deploy("contract.testnet".parse()?)
    // ///     .use_global_hash("DxfRbrjT3QPmoANMDYTR6iXPGJr7xRUyDnQhcAWjcoFF".parse()?)
    // ///     .without_init_call()
    // ///     .with_signer(signer)
    // ///     .send_to_testnet()
    // ///     .await?;
    // /// # Ok(())
    // /// # }
    pub fn use_global_hash(self, global_hash: CryptoHash) -> SetDeployActionBuilder {
        SetDeployActionBuilder::new(
            self.contract,
            Action::UseGlobalContract(Box::new(UseGlobalContractAction {
                contract_identifier: GlobalContractIdentifier::CodeHash(global_hash.into()),
            })),
        )
    }

    // /// Prepares a transaction to deploy a contract to the provided account using a mutable account-id reference to the code from the global contract code storage.
    // ///
    // /// Please note that you have to trust the account-id that you are providing. As the code is mutable, the owner of the referenced account can
    // /// change the code at any time which might lead to unexpected behavior or malicious activity.
    // ///
    // /// # Example
    // /// ```rust,no_run
    // /// use near_api::*;
    // ///
    // /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    // /// let signer = Signer::new(Signer::from_ledger())?;
    // /// let result = Contract::deploy("contract.testnet".parse()?)
    // ///     .use_global_account_id("nft-contract.testnet".parse()?)
    // ///     .without_init_call()
    // ///     .with_signer(signer)
    // ///     .send_to_testnet()
    // ///     .await?;
    // /// # Ok(())
    // /// # }
    pub fn use_global_account_id(self, global_account_id: AccountId) -> SetDeployActionBuilder {
        SetDeployActionBuilder::new(
            self.contract,
            Action::UseGlobalContract(Box::new(UseGlobalContractAction {
                contract_identifier: GlobalContractIdentifier::AccountId(global_account_id),
            })),
        )
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct SetDeployActionBuilder {
    contract: AccountId,
    deploy_action: Action,
}

impl SetDeployActionBuilder {
    pub const fn new(contract: AccountId, deploy_action: Action) -> Self {
        Self {
            contract,
            deploy_action,
        }
    }

    /// Prepares a transaction to deploy a contract to the provided account without an init call.
    ///
    /// This will deploy the contract without calling any function.
    pub fn without_init_call(self) -> ConstructTransaction {
        Transaction::construct(self.contract.clone(), self.contract).add_action(self.deploy_action)
    }

    /// Prepares a transaction to deploy a contract to the provided account with an init call.
    ///
    /// This will deploy the contract and call the init function with the provided arguments as a single transaction.
    pub fn with_init_call<Args: Serialize>(
        self,
        method_name: &str,
        args: Args,
    ) -> Result<SetDeployActionWithInitCallBuilder, BuilderError> {
        let args = serde_json::to_vec(&args)?;

        Ok(SetDeployActionWithInitCallBuilder::new(
            self.contract.clone(),
            method_name.to_string(),
            args,
            self.deploy_action,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct SetDeployActionWithInitCallBuilder {
    contract: AccountId,
    method_name: String,
    args: Vec<u8>,
    deploy_action: Action,
    gas: Option<NearGas>,
    deposit: Option<NearToken>,
}

impl SetDeployActionWithInitCallBuilder {
    const fn new(
        contract: AccountId,
        method_name: String,
        args: Vec<u8>,
        deploy_action: Action,
    ) -> Self {
        Self {
            contract,
            method_name,
            args,
            deploy_action,
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
            .add_action(self.deploy_action)
            .add_action(Action::FunctionCall(Box::new(FunctionCallAction {
                method_name: self.method_name.to_owned(),
                args: self.args,
                gas: gas.as_gas().into(),
                deposit: deposit.as_yoctonear().into(),
            })))
            .with_signer(signer)
    }
}

#[derive(Clone, Debug)]
pub struct GlobalDeployBuilder {
    code: Vec<u8>,
}

impl GlobalDeployBuilder {
    pub const fn new(code: Vec<u8>) -> Self {
        Self { code }
    }

    /// Prepares a transaction to deploy a code to the global contract code storage and reference it by hash.
    ///
    /// The code is immutable and cannot be changed once deployed.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = std::fs::read("path/to/your/contract.wasm")?;
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// let result = Contract::deploy_global_contract_code(code)
    ///     .as_hash()
    ///     .with_signer("some-account.testnet".parse()?, signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    #[allow(clippy::wrong_self_convention)]
    pub fn as_hash(self) -> SelfActionBuilder {
        SelfActionBuilder::new().add_action(Action::DeployGlobalContract(
            DeployGlobalContractAction {
                code: self.code.into(),
                deploy_mode: GlobalContractDeployMode::CodeHash,
            },
        ))
    }

    /// Prepares a transaction to deploy a code to the global contract code storage and reference it by account-id.
    ///
    /// You would be able to change the code later on.
    /// Please note that every subsequent upgrade will charge full deployment cost.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = std::fs::read("path/to/your/contract.wasm")?;
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// let result = Contract::deploy_global_contract_code(code)
    ///     .as_account_id("some-account.testnet".parse()?)
    ///     .with_signer(signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    #[allow(clippy::wrong_self_convention)]
    pub fn as_account_id(self, signer_id: AccountId) -> ConstructTransaction {
        Transaction::construct(signer_id.clone(), signer_id).add_action(
            Action::DeployGlobalContract(DeployGlobalContractAction {
                code: self.code.into(),
                deploy_mode: GlobalContractDeployMode::AccountId,
            }),
        )
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
        let request = QueryRequest::CallFunction {
            account_id: self.contract,
            method_name: self.method_name,
            args_base64: FunctionArgs(to_base64(&self.args)),
        };

        QueryBuilder::new(
            SimpleQueryRpc { request },
            Reference::Optimistic,
            CallResultHandler::<Response>::new(),
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
                gas: gas.as_gas().into(),
                deposit: deposit.as_yoctonear().into(),
            },
        )))
    }
}
