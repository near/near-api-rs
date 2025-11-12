use std::sync::Arc;

use borsh::BorshDeserialize;
use near_api_types::{
    contract::ContractSourceMetadata,
    transaction::actions::{
        DeployContractAction, DeployGlobalContractAction, FunctionCallAction,
        GlobalContractDeployMode, GlobalContractIdentifier, UseGlobalContractAction,
    },
    AccountId, Action, CryptoHash, Data, FunctionArgs, NearGas, NearToken, Reference, StoreKey,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    advanced::{query_request::QueryRequest, query_rpc::SimpleQueryRpc},
    common::{
        query::{
            CallResultBorshHandler, CallResultHandler, PostprocessHandler, RequestBuilder,
            ViewCodeHandler, ViewStateHandler,
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
    /// Returns the underlying account ID for this contract.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = Contract("contract.testnet".parse()?);
    /// let account_id = contract.account_id();
    /// println!("Contract account ID: {}", account_id);
    /// # Ok(())
    /// # }
    /// ```
    pub const fn account_id(&self) -> &AccountId {
        &self.0
    }

    /// Converts this contract to an Account for account-related operations.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = Contract("contract.testnet".parse()?);
    /// let account = contract.as_account();
    /// let account_info = account.view().fetch_from_testnet().await?;
    /// println!("Account balance: {}", account_info.data.amount);
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_account(&self) -> crate::account::Account {
        crate::account::Account(self.0.clone())
    }

    /// Creates a StorageDeposit wrapper for storage management operations on this contract.
    ///
    /// This is useful for contracts that implement the [NEP-145](https://github.com/near/NEPs/blob/master/neps/nep-0145.md) storage management standard.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = Contract("usdt.tether-token.near".parse()?);
    /// let storage = contract.storage_deposit();
    ///
    /// // Check storage balance for an account
    /// let balance = storage.view_account_storage("alice.near".parse()?)?.fetch_from_mainnet().await?;
    /// println!("Storage balance: {:?}", balance);
    /// # Ok(())
    /// # }
    /// ```
    pub fn storage_deposit(&self) -> crate::StorageDeposit {
        crate::StorageDeposit::on_contract(self.0.clone())
    }

    /// Prepares a call to a contract function with JSON-serialized arguments.
    ///
    /// This is the default and most common way to call contract functions, using JSON serialization
    /// for the input arguments. This will return a builder that can be used to prepare a query or a transaction.
    ///
    /// For alternative serialization formats, see:
    /// - [`call_function_borsh`](Contract::call_function_borsh) for Borsh serialization
    /// - [`call_function_raw`](Contract::call_function_raw) for pre-serialized raw bytes
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

    /// Prepares a call to a contract function with Borsh-serialized arguments.
    ///
    /// This method is useful when the contract expects Borsh-encoded input arguments instead of JSON.
    /// This is less common but can be more efficient for certain use cases.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    /// use borsh::BorshSerialize;
    ///
    /// #[derive(BorshSerialize)]
    /// struct MyArgs {
    ///     value: u64,
    /// }
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// let args = MyArgs { value: 42 };
    /// let result = Contract("some_contract.testnet".parse()?)
    ///     .call_function_borsh("set_value", args)?
    ///     .transaction()
    ///     .with_signer("alice.testnet".parse()?, signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn call_function_borsh<Args>(
        &self,
        method_name: &str,
        args: Args,
    ) -> Result<CallFunctionBuilder, std::io::Error>
    where
        Args: borsh::BorshSerialize,
    {
        let args = borsh::to_vec(&args)?;

        Ok(CallFunctionBuilder {
            contract: self.0.clone(),
            method_name: method_name.to_string(),
            args,
        })
    }

    /// Prepares a call to a contract function with pre-serialized raw bytes.
    ///
    /// This method is useful when you already have serialized arguments or need complete control
    /// over the serialization format. The bytes are passed directly to the contract without any
    /// additional processing.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// // Pre-serialized or custom-encoded data
    /// let raw_args = vec![1, 2, 3, 4];
    /// let result = Contract("some_contract.testnet".parse()?)
    ///     .call_function_raw("custom_method", raw_args)
    ///     .transaction()
    ///     .with_signer("alice.testnet".parse()?, signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn call_function_raw(&self, method_name: &str, args: Vec<u8>) -> CallFunctionBuilder {
        CallFunctionBuilder {
            contract: self.0.clone(),
            method_name: method_name.to_string(),
            args,
        }
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

    /// Publishes contract code to the global contract code storage.
    ///
    /// This will allow other users to reference given code as hash or account-id and reduce
    /// the gas cost for deployment.
    ///
    /// Please be aware that the deploy costs 10x more compared to the regular costs and the tokens are burnt
    /// with no way to get it back.
    ///
    /// ## Example publishing contract code as immutable hash
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = std::fs::read("path/to/your/contract.wasm")?;
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// let result = Contract::publish_contract(code)
    ///     .as_hash()
    ///     .with_signer("some-account.testnet".parse()?, signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Example publishing contract code as mutable account-id
    ///
    /// The account-id version is upgradable and can be changed.
    /// Please note that every subsequent upgrade will charge full deployment cost.
    ///
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = std::fs::read("path/to/your/contract.wasm")?;
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// let result = Contract::publish_contract(code)
    ///     .as_account("nft-contract.testnet".parse()?)
    ///     .with_signer(signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub const fn publish_contract(code: Vec<u8>) -> PublishContractBuilder {
        PublishContractBuilder::new(code)
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
    #[deprecated(
        since = "0.8.0",
        note = "Use `publish_contract(code).as_hash()` for hash-based deployment or `publish_contract(code).as_account(account_id)` for account-based deployment"
    )]
    pub const fn deploy_global_contract_code(code: Vec<u8>) -> GlobalDeployBuilder {
        GlobalDeployBuilder::new(code)
    }

    /// Prepares a query to fetch the [ABI](near_api_types::abi::AbiRoot) of the contract using the following [standard](https://github.com/near/near-abi-rs).
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
    ) -> RequestBuilder<
        PostprocessHandler<Option<near_api_types::abi::AbiRoot>, CallResultHandler<Vec<u8>>>,
    > {
        self.call_function("__contract_abi", ())
            .expect("arguments are always serializable")
            .read_only()
            .map(|data: Data<Vec<u8>>| {
                serde_json::from_slice(zstd::decode_all(data.data.as_slice()).ok()?.as_slice()).ok()
            })
    }

    /// Prepares a query to fetch the wasm code ([Data]<[ContractCodeView](near_api_types::ContractCodeView)>) of the contract.
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
    pub fn wasm(&self) -> RequestBuilder<ViewCodeHandler> {
        let request = QueryRequest::ViewCode {
            account_id: self.0.clone(),
        };

        RequestBuilder::new(
            SimpleQueryRpc { request },
            Reference::Optimistic,
            ViewCodeHandler,
        )
    }

    /// Creates a builder to query contract code from the global contract code storage.
    ///
    /// The global contract code storage allows contracts to be deployed once and referenced
    /// by multiple accounts, reducing deployment costs. This feature is defined in [NEP-591](https://github.com/near/NEPs/blob/2f6b702d55a4cd470b50d35e2f3fde6e0fb4dced/neps/nep-0591.md).
    /// Contracts can be referenced either by a contract hash (immutable) or by an account ID (mutable).
    ///
    /// # Example querying by account ID
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = Contract::global_wasm()
    ///     .by_account_id("nft-contract.testnet".parse()?)
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Global contract code: {}", code.data.code_base64);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Example querying by hash
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = Contract::global_wasm()
    ///     .by_hash("DxfRbrjT3QPmoANMDYTR6iXPGJr7xRUyDnQhcAWjcoFF".parse()?)
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Global contract code: {}", code.data.code_base64);
    /// # Ok(())
    /// # }
    /// ```
    pub const fn global_wasm() -> GlobalWasmBuilder {
        GlobalWasmBuilder
    }

    /// Prepares a query to fetch the storage of the contract ([Data]<[ViewStateResult](near_api_types::ViewStateResult)>) using the given prefix as a filter.
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
    pub fn view_storage_with_prefix(&self, prefix: &[u8]) -> RequestBuilder<ViewStateHandler> {
        let request = QueryRequest::ViewState {
            account_id: self.0.clone(),
            prefix_base64: StoreKey(to_base64(prefix)),
            include_proof: Some(false),
        };

        RequestBuilder::new(
            SimpleQueryRpc { request },
            Reference::Optimistic,
            ViewStateHandler,
        )
    }

    /// Prepares a query to fetch the storage of the contract ([Data]<[ViewStateResult](near_api_types::ViewStateResult)>).
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
    pub fn view_storage(&self) -> RequestBuilder<ViewStateHandler> {
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
    ) -> RequestBuilder<CallResultHandler<ContractSourceMetadata>> {
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

    /// Deploys a contract from previously published code in the global contract code storage.
    ///
    /// Accepts either a code hash (immutable) or account ID (mutable) reference.
    ///
    /// # Example deploying from hash
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// let code_hash: near_api::CryptoHash = "DxfRbrjT3QPmoANMDYTR6iXPGJr7xRUyDnQhcAWjcoFF".parse()?;
    /// let result = Contract::deploy("contract.testnet".parse()?)
    ///     .deploy_from_published(code_hash)
    ///     .without_init_call()
    ///     .with_signer(signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Example deploying from account ID
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let signer = Signer::new(Signer::from_ledger())?;
    /// let result = Contract::deploy("contract.testnet".parse()?)
    ///     .deploy_from_published("nft-contract.testnet".parse()?)
    ///     .without_init_call()
    ///     .with_signer(signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn deploy_from_published(
        self,
        reference: impl IntoGlobalContractRef,
    ) -> SetDeployActionBuilder {
        let identifier = reference.into_identifier();
        SetDeployActionBuilder::new(
            self.contract,
            Action::UseGlobalContract(Box::new(UseGlobalContractAction {
                contract_identifier: identifier,
            })),
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
    #[deprecated(
        since = "0.8.0",
        note = "Use `deploy_from_published(code_hash)` instead"
    )]
    pub fn use_global_hash(self, global_hash: CryptoHash) -> SetDeployActionBuilder {
        SetDeployActionBuilder::new(
            self.contract,
            Action::UseGlobalContract(Box::new(UseGlobalContractAction {
                contract_identifier: GlobalContractIdentifier::CodeHash(global_hash),
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
    #[deprecated(
        since = "0.8.0",
        note = "Use `deploy_from_published(account_id)` instead"
    )]
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

    /// Specify the gas limit for the transaction to the maximum allowed value.
    pub const fn max_gas(mut self) -> Self {
        self.gas = Some(NearGas::from_tgas(300));
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
                gas,
                deposit,
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
                code: self.code,
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
                code: self.code,
                deploy_mode: GlobalContractDeployMode::AccountId,
            }),
        )
    }
}

/// Builder for publishing contract code to the global contract code storage.
///
/// Created by [`Contract::publish_contract`]. Choose to publish as:
/// - Immutable (hash-based) via [`as_hash`](Self::as_hash)
/// - Mutable (account-based) via [`as_account`](Self::as_account)
#[derive(Clone, Debug)]
pub struct PublishContractBuilder {
    code: Vec<u8>,
}

impl PublishContractBuilder {
    pub const fn new(code: Vec<u8>) -> Self {
        Self { code }
    }

    /// Publishes the contract code as immutable hash.
    ///
    /// The code will be indexed by its hash and cannot be changed.
    /// Any account can publish hash-based code.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = std::fs::read("path/to/your/contract.wasm")?;
    /// let signer = Signer::new(Signer::from_ledger())?;
    ///
    /// Contract::publish_contract(code)
    ///     .as_hash()
    ///     .with_signer("publisher.testnet".parse()?, signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)]
    pub fn as_hash(self) -> SelfActionBuilder {
        let action = Action::DeployGlobalContract(DeployGlobalContractAction {
            code: self.code,
            deploy_mode: GlobalContractDeployMode::CodeHash,
        });

        SelfActionBuilder::new().add_action(action)
    }

    /// Publishes the contract code as mutable account-based deployment.
    ///
    /// The code will be indexed by the specified account and can be upgraded later.
    /// The transaction is automatically bound to the provided account.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = std::fs::read("path/to/your/contract.wasm")?;
    /// let signer = Signer::new(Signer::from_ledger())?;
    ///
    /// Contract::publish_contract(code)
    ///     .as_account("nft-template.testnet".parse()?)
    ///     .with_signer(signer)
    ///     .send_to_testnet()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)]
    pub fn as_account(self, account_id: AccountId) -> ConstructTransaction {
        let action = Action::DeployGlobalContract(DeployGlobalContractAction {
            code: self.code,
            deploy_mode: GlobalContractDeployMode::AccountId,
        });

        Transaction::construct(account_id.clone(), account_id).add_action(action)
    }
}

/// Trait for types that can reference global contracts.
///
/// This trait allows the [`DeployBuilder::deploy_from_published`] method to accept
/// either a code hash (immutable) or an account ID (mutable) reference to global
/// contract code.
///
/// # Implementations
/// - [`CryptoHash`] - References immutable global contract by hash
/// - [`AccountId`] - References mutable global contract by account ID
/// - [`GlobalContractIdentifier`] - Direct identifier (for advanced usage)
pub trait IntoGlobalContractRef {
    /// Converts this type into a [`GlobalContractIdentifier`].
    fn into_identifier(self) -> GlobalContractIdentifier;
}

impl IntoGlobalContractRef for CryptoHash {
    fn into_identifier(self) -> GlobalContractIdentifier {
        GlobalContractIdentifier::CodeHash(self)
    }
}

impl IntoGlobalContractRef for AccountId {
    fn into_identifier(self) -> GlobalContractIdentifier {
        GlobalContractIdentifier::AccountId(self)
    }
}

impl IntoGlobalContractRef for GlobalContractIdentifier {
    fn into_identifier(self) -> GlobalContractIdentifier {
        self
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
    ) -> RequestBuilder<CallResultHandler<Response>> {
        let request = QueryRequest::CallFunction {
            account_id: self.contract,
            method_name: self.method_name,
            args_base64: FunctionArgs(to_base64(&self.args)),
        };

        RequestBuilder::new(
            SimpleQueryRpc { request },
            Reference::Optimistic,
            CallResultHandler::<Response>::new(),
        )
    }

    /// Prepares a read-only query that deserializes the response using Borsh instead of JSON.
    ///
    /// This method is useful when the contract returns Borsh-encoded data instead of JSON.
    ///
    /// ## Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let value: Data<u64> = Contract("some_contract.testnet".parse()?)
    ///     .call_function("get_number", ())?
    ///     .read_only_borsh()
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Value: {:?}", value);
    /// # Ok(())
    /// # }
    /// ```
    pub fn read_only_borsh<Response: Send + Sync + BorshDeserialize>(
        self,
    ) -> RequestBuilder<CallResultBorshHandler<Response>> {
        let request = QueryRequest::CallFunction {
            account_id: self.contract,
            method_name: self.method_name,
            args_base64: FunctionArgs(to_base64(&self.args)),
        };

        RequestBuilder::new(
            SimpleQueryRpc { request },
            Reference::Optimistic,
            CallResultBorshHandler::<Response>::new(),
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

    /// Specify the max gas for the transaction. By default it is set to 300 TGas.
    pub const fn max_gas(mut self) -> Self {
        self.gas = Some(NearGas::from_tgas(300));
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
                gas,
                deposit,
            },
        )))
    }
}

/// Builder for querying contract code from the global contract code storage defined in [NEP-591](https://github.com/near/NEPs/blob/2f6b702d55a4cd470b50d35e2f3fde6e0fb4dced/neps/nep-0591.md).
pub struct GlobalWasmBuilder;

impl GlobalWasmBuilder {
    /// Prepares a query to fetch global contract code ([Data]<[ContractCodeView](near_api_types::ContractCodeView)>) by account ID.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = Contract::global_wasm()
    ///     .by_account_id("nft-contract.testnet".parse()?)
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Code: {}", code.data.code_base64);
    /// # Ok(())
    /// # }
    /// ```
    pub fn by_account_id(&self, account_id: AccountId) -> RequestBuilder<ViewCodeHandler> {
        let request = QueryRequest::ViewGlobalContractCodeByAccountId { account_id };

        RequestBuilder::new(
            SimpleQueryRpc { request },
            Reference::Optimistic,
            ViewCodeHandler,
        )
    }

    /// Prepares a query to fetch global contract code ([Data]<[ContractCodeView](near_api_types::ContractCodeView)>) by hash.
    ///
    /// # Example
    /// ```rust,no_run
    /// use near_api::*;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let code = Contract::global_wasm()
    ///     .by_hash("DxfRbrjT3QPmoANMDYTR6iXPGJr7xRUyDnQhcAWjcoFF".parse()?)
    ///     .fetch_from_testnet()
    ///     .await?;
    /// println!("Code: {}", code.data.code_base64);
    /// # Ok(())
    /// # }
    /// ```
    pub fn by_hash(&self, code_hash: CryptoHash) -> RequestBuilder<ViewCodeHandler> {
        let request = QueryRequest::ViewGlobalContractCode { code_hash };

        RequestBuilder::new(
            SimpleQueryRpc { request },
            Reference::Optimistic,
            ViewCodeHandler,
        )
    }
}
