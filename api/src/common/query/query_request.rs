use near_api_types::{AccountId, CryptoHash, Reference};
use near_openrpc_client::{FunctionArgs, StoreKey};
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
        public_key: near_openrpc_client::PublicKey,
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

fn apply_reference(req: &mut serde_json::Value, reference: &Reference) {
    match reference {
        Reference::Final => {
            req["finality"] = serde_json::Value::String("final".to_string());
        }
        Reference::NearFinal => {
            req["finality"] = serde_json::Value::String("near-final".to_string());
        }
        Reference::Optimistic => {
            req["finality"] = serde_json::Value::String("optimistic".to_string());
        }
        Reference::AtBlock(height) => {
            req["block_id"] = serde_json::json!(*height);
        }
        Reference::AtBlockHash(hash) => {
            req["block_id"] = serde_json::Value::String(hash.to_string());
        }
    }
}

impl QueryRequest {
    /// Convert this simplified query request to a JSON value suitable for the "query" RPC method.
    pub fn to_rpc_query_request(self, reference: Reference) -> serde_json::Value {
        match self {
            Self::ViewAccount { account_id } => {
                let mut req = serde_json::json!({
                    "request_type": "view_account",
                    "account_id": account_id,
                });
                apply_reference(&mut req, &reference);
                req
            }
            Self::ViewCode { account_id } => {
                let mut req = serde_json::json!({
                    "request_type": "view_code",
                    "account_id": account_id,
                });
                apply_reference(&mut req, &reference);
                req
            }
            Self::ViewState {
                account_id,
                include_proof,
                prefix_base64,
            } => {
                let mut req = serde_json::json!({
                    "request_type": "view_state",
                    "account_id": account_id,
                    "prefix_base64": prefix_base64,
                });
                if let Some(include_proof) = include_proof {
                    req["include_proof"] = serde_json::json!(include_proof);
                }
                apply_reference(&mut req, &reference);
                req
            }
            Self::ViewAccessKey {
                account_id,
                public_key,
            } => {
                let mut req = serde_json::json!({
                    "request_type": "view_access_key",
                    "account_id": account_id,
                    "public_key": public_key,
                });
                apply_reference(&mut req, &reference);
                req
            }
            Self::ViewAccessKeyList { account_id } => {
                let mut req = serde_json::json!({
                    "request_type": "view_access_key_list",
                    "account_id": account_id,
                });
                apply_reference(&mut req, &reference);
                req
            }
            Self::CallFunction {
                account_id,
                method_name,
                args_base64,
            } => {
                let mut req = serde_json::json!({
                    "request_type": "call_function",
                    "account_id": account_id,
                    "method_name": method_name,
                    "args_base64": args_base64,
                });
                apply_reference(&mut req, &reference);
                req
            }
            Self::ViewGlobalContractCode { code_hash } => {
                let mut req = serde_json::json!({
                    "request_type": "view_global_contract_code",
                    "code_hash": code_hash.to_string(),
                });
                apply_reference(&mut req, &reference);
                req
            }
            Self::ViewGlobalContractCodeByAccountId { account_id } => {
                let mut req = serde_json::json!({
                    "request_type": "view_global_contract_code_by_account_id",
                    "account_id": account_id,
                });
                apply_reference(&mut req, &reference);
                req
            }
        }
    }
}
