use near_openapi_client::types::{
    BlockId, CallFunctionByBlockIdRequestType, CallFunctionByFinalityRequestType, Finality,
    FunctionArgs, PublicKey, RpcQueryRequest, StoreKey, ViewAccessKeyByBlockIdRequestType,
    ViewAccessKeyByFinalityRequestType, ViewAccessKeyListByBlockIdRequestType,
    ViewAccessKeyListByFinalityRequestType, ViewAccountByBlockIdRequestType,
    ViewAccountByFinalityRequestType, ViewCodeByBlockIdRequestType, ViewCodeByFinalityRequestType,
    ViewGlobalContractCodeByAccountIdByBlockIdRequestType,
    ViewGlobalContractCodeByAccountIdByFinalityRequestType,
    ViewGlobalContractCodeByBlockIdRequestType, ViewGlobalContractCodeByFinalityRequestType,
    ViewStateByBlockIdRequestType, ViewStateByFinalityRequestType,
};
use near_types::{AccountId, CryptoHash, Reference};
use serde::{Deserialize, Serialize};

/// Simplified query request structure that eliminates duplication by removing reference types
/// from the enum variants. The reference is provided separately when converting to RPC format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryRequest {
    /// View account information
    ViewAccount { account_id: AccountId },
    /// View contract code for an account
    ViewCode { account_id: AccountId },
    /// View state of an account with optional proof and key prefix
    ViewState {
        account_id: AccountId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        include_proof: Option<bool>,
        prefix_base64: StoreKey,
    },
    /// View a specific access key for an account
    ViewAccessKey {
        account_id: AccountId,
        public_key: PublicKey,
    },
    /// View all access keys for an account
    ViewAccessKeyList { account_id: AccountId },
    /// Call a view function on a contract
    CallFunction {
        account_id: AccountId,
        method_name: String,
        args_base64: FunctionArgs,
    },
    /// View global contract code by hash
    ViewGlobalContractCode { code_hash: CryptoHash },
    /// View global contract code by account ID
    ViewGlobalContractCodeByAccountId { account_id: AccountId },
}

impl QueryRequest {
    /// Convert this simplified query request to the OpenAPI RpcQueryRequest enum
    /// This method handles the conversion from our unified Reference type to the appropriate
    /// OpenAPI variant based on the reference type
    pub fn to_rpc_query_request(
        self,
        reference: Reference,
    ) -> near_openapi_client::types::RpcQueryRequest {
        match self {
            QueryRequest::ViewAccount { account_id } => {
                match reference {
                    Reference::Final => RpcQueryRequest::ViewAccountByFinality {
                        account_id,
                        finality: Finality::Final,
                        request_type: ViewAccountByFinalityRequestType::ViewAccount,
                    },
                    Reference::NearFinal => RpcQueryRequest::ViewAccountByFinality {
                        account_id,
                        finality: Finality::NearFinal,
                        request_type: ViewAccountByFinalityRequestType::ViewAccount,
                    },
                    Reference::Optimistic => RpcQueryRequest::ViewAccountByFinality {
                        account_id,
                        finality: Finality::Optimistic,
                        request_type: ViewAccountByFinalityRequestType::ViewAccount,
                    },
                    Reference::AtBlock(height) => RpcQueryRequest::ViewAccountByBlockId {
                        account_id,
                        block_id: BlockId::BlockHeight(height),
                        request_type: ViewAccountByBlockIdRequestType::ViewAccount,
                    },
                    Reference::AtBlockHash(hash) => RpcQueryRequest::ViewAccountByBlockId {
                        account_id,
                        block_id: BlockId::CryptoHash(hash.into()),
                        request_type: ViewAccountByBlockIdRequestType::ViewAccount,
                    },
                }
            },
            QueryRequest::ViewCode { account_id } => {
                match reference {
                    Reference::Final => RpcQueryRequest::ViewCodeByFinality {
                        account_id,
                        finality: Finality::Final,
                        request_type: ViewCodeByFinalityRequestType::ViewCode,
                    },
                    Reference::NearFinal => RpcQueryRequest::ViewCodeByFinality {
                        account_id,
                        finality: Finality::NearFinal,
                        request_type: ViewCodeByFinalityRequestType::ViewCode,
                    },
                    Reference::Optimistic => RpcQueryRequest::ViewCodeByFinality {
                        account_id,
                        finality: Finality::Optimistic,
                        request_type: ViewCodeByFinalityRequestType::ViewCode,
                    },
                    Reference::AtBlock(height) => RpcQueryRequest::ViewCodeByBlockId {
                        account_id,
                        block_id: BlockId::BlockHeight(height),
                        request_type: ViewCodeByBlockIdRequestType::ViewCode,
                    },
                    Reference::AtBlockHash(hash) => RpcQueryRequest::ViewCodeByBlockId {
                        account_id,
                        block_id: BlockId::CryptoHash(hash.into()),
                        request_type: ViewCodeByBlockIdRequestType::ViewCode,
                    },
                }
            },
            QueryRequest::ViewState { account_id, include_proof, prefix_base64 } => {
                match reference {
                    Reference::Final => RpcQueryRequest::ViewStateByFinality {
                        account_id,
                        finality: Finality::Final,
                        include_proof,
                        prefix_base64,
                        request_type: ViewStateByFinalityRequestType::ViewState,
                    },
                    Reference::NearFinal => RpcQueryRequest::ViewStateByFinality {
                        account_id,
                        finality: Finality::NearFinal,
                        include_proof,
                        prefix_base64,
                        request_type: ViewStateByFinalityRequestType::ViewState,
                    },
                    Reference::Optimistic => RpcQueryRequest::ViewStateByFinality {
                        account_id,
                        finality: Finality::Optimistic,
                        include_proof,
                        prefix_base64,
                        request_type: ViewStateByFinalityRequestType::ViewState,
                    },
                    Reference::AtBlock(height) => RpcQueryRequest::ViewStateByBlockId {
                        account_id,
                        block_id: BlockId::BlockHeight(height),
                        include_proof,
                        prefix_base64,
                        request_type: ViewStateByBlockIdRequestType::ViewState,
                    },
                    Reference::AtBlockHash(hash) => RpcQueryRequest::ViewStateByBlockId {
                        account_id,
                        block_id: BlockId::CryptoHash(hash.into()),
                        include_proof,
                        prefix_base64,
                        request_type: ViewStateByBlockIdRequestType::ViewState,
                    },
                }
            },
            QueryRequest::ViewAccessKey { account_id, public_key } => {
                match reference {
                    Reference::Final => RpcQueryRequest::ViewAccessKeyByFinality {
                        account_id,
                        public_key,
                        finality: Finality::Final,
                        request_type: ViewAccessKeyByFinalityRequestType::ViewAccessKey,
                    },
                    Reference::Optimistic => RpcQueryRequest::ViewAccessKeyByFinality {
                        account_id,
                        public_key,
                        finality: Finality::Optimistic,
                        request_type: ViewAccessKeyByFinalityRequestType::ViewAccessKey,
                    },
                    Reference::NearFinal => RpcQueryRequest::ViewAccessKeyByFinality {
                        account_id,
                        public_key,
                        finality: Finality::NearFinal,
                        request_type: ViewAccessKeyByFinalityRequestType::ViewAccessKey,
                    },
                    Reference::AtBlock(height) => RpcQueryRequest::ViewAccessKeyByBlockId {
                        account_id,
                        public_key,
                        block_id: BlockId::BlockHeight(height),
                        request_type: ViewAccessKeyByBlockIdRequestType::ViewAccessKey,
                    },
                    Reference::AtBlockHash(hash) => RpcQueryRequest::ViewAccessKeyByBlockId {
                        account_id,
                        public_key,
                        block_id: BlockId::CryptoHash(hash.into()),
                        request_type: ViewAccessKeyByBlockIdRequestType::ViewAccessKey,
                    },
                }
            },
            QueryRequest::ViewAccessKeyList { account_id } => {
                match reference {
                    Reference::Final => RpcQueryRequest::ViewAccessKeyListByFinality {
                        account_id,
                        finality: Finality::Final,
                        request_type: ViewAccessKeyListByFinalityRequestType::ViewAccessKeyList,
                    },
                    Reference::Optimistic => RpcQueryRequest::ViewAccessKeyListByFinality {
                        account_id,
                        finality: Finality::Optimistic,
                        request_type: ViewAccessKeyListByFinalityRequestType::ViewAccessKeyList,
                    },
                    Reference::NearFinal => RpcQueryRequest::ViewAccessKeyListByFinality {
                        account_id,
                        finality: Finality::NearFinal,
                        request_type: ViewAccessKeyListByFinalityRequestType::ViewAccessKeyList,
                    },
                    Reference::AtBlock(height) => RpcQueryRequest::ViewAccessKeyListByBlockId {
                        account_id,
                        block_id: BlockId::BlockHeight(height),
                        request_type: ViewAccessKeyListByBlockIdRequestType::ViewAccessKeyList,
                    },
                    Reference::AtBlockHash(hash) => RpcQueryRequest::ViewAccessKeyListByBlockId {
                        account_id,
                        block_id: BlockId::CryptoHash(hash.into()),
                        request_type: ViewAccessKeyListByBlockIdRequestType::ViewAccessKeyList,
                    },
                }
            },
            QueryRequest::CallFunction { account_id, method_name, args_base64 } => {
                match reference {
                    Reference::Final => RpcQueryRequest::CallFunctionByFinality {
                        account_id,
                        method_name,
                        args_base64,
                        finality: Finality::Final,
                        request_type: CallFunctionByFinalityRequestType::CallFunction,
                    },
                    Reference::Optimistic => RpcQueryRequest::CallFunctionByFinality {
                        account_id,
                        method_name,
                        args_base64,
                        finality: Finality::Optimistic,
                        request_type: CallFunctionByFinalityRequestType::CallFunction,
                    },
                    Reference::NearFinal => RpcQueryRequest::CallFunctionByFinality {
                        account_id,
                        method_name,
                        args_base64,
                        finality: Finality::NearFinal,
                        request_type: CallFunctionByFinalityRequestType::CallFunction,
                    },
                    Reference::AtBlock(height) => RpcQueryRequest::CallFunctionByBlockId {
                        account_id,
                        method_name,
                        args_base64,
                        block_id: BlockId::BlockHeight(height),
                        request_type: CallFunctionByBlockIdRequestType::CallFunction,
                    },
                    Reference::AtBlockHash(hash) => RpcQueryRequest::CallFunctionByBlockId {
                        account_id,
                        method_name,
                        args_base64,
                        block_id: BlockId::CryptoHash(hash.into()),
                        request_type: CallFunctionByBlockIdRequestType::CallFunction,
                    },
                }
            },
            QueryRequest::ViewGlobalContractCode { code_hash } => {
                match reference {
                    Reference::Final => RpcQueryRequest::ViewGlobalContractCodeByFinality {
                        code_hash: code_hash.into(),
                        finality: Finality::Final,
                        request_type: ViewGlobalContractCodeByFinalityRequestType::ViewGlobalContractCode,
                    },
                    Reference::Optimistic => RpcQueryRequest::ViewGlobalContractCodeByFinality {
                        code_hash: code_hash.into(),
                        finality: Finality::Optimistic,
                        request_type: ViewGlobalContractCodeByFinalityRequestType::ViewGlobalContractCode,
                    },
                    Reference::NearFinal => RpcQueryRequest::ViewGlobalContractCodeByFinality {
                        code_hash: code_hash.into(),
                        finality: Finality::NearFinal,
                        request_type: ViewGlobalContractCodeByFinalityRequestType::ViewGlobalContractCode,
                    },
                    Reference::AtBlock(height) => RpcQueryRequest::ViewGlobalContractCodeByBlockId {
                        code_hash: code_hash.into(),
                        block_id: BlockId::BlockHeight(height),
                        request_type: ViewGlobalContractCodeByBlockIdRequestType::ViewGlobalContractCode,
                    },
                    Reference::AtBlockHash(hash) => RpcQueryRequest::ViewGlobalContractCodeByBlockId {
                        code_hash: code_hash.into(),
                        block_id: BlockId::CryptoHash(hash.into()),
                        request_type: ViewGlobalContractCodeByBlockIdRequestType::ViewGlobalContractCode,
                    },
                }
            },
            QueryRequest::ViewGlobalContractCodeByAccountId { account_id } => {
                match reference {
                    Reference::Final => RpcQueryRequest::ViewGlobalContractCodeByAccountIdByFinality {
                        account_id,
                        finality: Finality::Final,
                        request_type: ViewGlobalContractCodeByAccountIdByFinalityRequestType::ViewGlobalContractCodeByAccountId,
                    },
                    Reference::Optimistic => RpcQueryRequest::ViewGlobalContractCodeByAccountIdByFinality {
                        account_id,
                        finality: Finality::Optimistic,
                        request_type: ViewGlobalContractCodeByAccountIdByFinalityRequestType::ViewGlobalContractCodeByAccountId,
                    },
                    Reference::NearFinal => RpcQueryRequest::ViewGlobalContractCodeByAccountIdByFinality {
                        account_id,
                        finality: Finality::NearFinal,
                        request_type: ViewGlobalContractCodeByAccountIdByFinalityRequestType::ViewGlobalContractCodeByAccountId,
                    },
                    Reference::AtBlock(height) => RpcQueryRequest::ViewGlobalContractCodeByAccountIdByBlockId {
                        account_id,
                        block_id: BlockId::BlockHeight(height),
                        request_type: ViewGlobalContractCodeByAccountIdByBlockIdRequestType::ViewGlobalContractCodeByAccountId,
                    },
                    Reference::AtBlockHash(hash) => RpcQueryRequest::ViewGlobalContractCodeByAccountIdByBlockId {
                        account_id,
                        block_id: BlockId::CryptoHash(hash.into()),
                        request_type: ViewGlobalContractCodeByAccountIdByBlockIdRequestType::ViewGlobalContractCodeByAccountId,
                    },
                }
            },
        }
    }
}
