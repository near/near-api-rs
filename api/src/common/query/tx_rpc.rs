use near_api_types::{AccountId, CryptoHash, TxExecutionStatus};
use near_openapi_client::Client;
use near_openapi_client::types::{
    ErrorWrapperForRpcLightClientProofError, ErrorWrapperForRpcReceiptError,
    JsonRpcRequestForExperimentalReceipt, JsonRpcRequestForExperimentalReceiptMethod,
    JsonRpcRequestForLightClientProof, JsonRpcRequestForLightClientProofMethod,
    JsonRpcRequestForTx, JsonRpcRequestForTxMethod,
    JsonRpcResponseForRpcLightClientExecutionProofResponseAndRpcLightClientProofError,
    JsonRpcResponseForRpcReceiptResponseAndRpcReceiptError,
    JsonRpcResponseForRpcTransactionResponseAndRpcTransactionError,
    RpcLightClientExecutionProofRequest, RpcLightClientExecutionProofRequestVariant0Type,
    RpcLightClientExecutionProofResponse, RpcLightClientProofError, RpcReceiptError,
    RpcReceiptRequest, RpcReceiptResponse, RpcTransactionError, RpcTransactionStatusRequest,
};

use crate::common::utils::to_retry_error;
use crate::{
    NetworkConfig,
    advanced::RpcType,
    common::utils::{
        is_critical_light_client_proof_error, is_critical_receipt_error,
        is_critical_transaction_error,
    },
    config::RetryResponse,
    errors::SendRequestError,
};

/// Reference type for transaction status queries.
///
/// Identifies a transaction by its hash, sender account, and the desired execution status to wait for.
#[derive(Clone, Debug)]
pub struct TransactionStatusRef {
    pub sender_account_id: AccountId,
    pub tx_hash: CryptoHash,
    pub wait_until: TxExecutionStatus,
}

/// RPC type for fetching transaction status by hash.
///
/// Uses the `tx` RPC method to query the status of a previously submitted transaction.
#[derive(Clone, Debug)]
pub struct TransactionStatusRpc;

#[async_trait::async_trait]
impl RpcType for TransactionStatusRpc {
    type RpcReference = TransactionStatusRef;
    type Response = near_openapi_client::types::RpcTransactionResponse;
    type Error = RpcTransactionError;

    async fn send_query(
        &self,
        client: &Client,
        _network: &NetworkConfig,
        reference: &TransactionStatusRef,
    ) -> RetryResponse<Self::Response, SendRequestError<RpcTransactionError>> {
        let response = client
            .tx(&JsonRpcRequestForTx {
                id: "0".to_string(),
                jsonrpc: "2.0".to_string(),
                method: JsonRpcRequestForTxMethod::Tx,
                params: RpcTransactionStatusRequest::Variant1 {
                    sender_account_id: reference.sender_account_id.clone(),
                    tx_hash: reference.tx_hash.into(),
                    wait_until: reference.wait_until,
                },
            })
            .await
            .map(|r| r.into_inner())
            .map_err(SendRequestError::from);

        match response {
            Ok(JsonRpcResponseForRpcTransactionResponseAndRpcTransactionError::Variant0 {
                result,
                ..
            }) => RetryResponse::Ok(result),
            Ok(JsonRpcResponseForRpcTransactionResponseAndRpcTransactionError::Variant1 {
                error,
                ..
            }) => {
                let error = SendRequestError::from(error);
                to_retry_error(error, is_critical_transaction_error)
            }
            Err(err) => to_retry_error(err, is_critical_transaction_error),
        }
    }
}

/// Reference type for receipt queries.
///
/// Identifies a receipt by its ID.
#[derive(Clone, Debug)]
pub struct ReceiptRef {
    pub receipt_id: CryptoHash,
}

/// RPC type for fetching a receipt by its ID.
///
/// Uses the `EXPERIMENTAL_receipt` RPC method.
#[derive(Clone, Debug)]
pub struct ReceiptRpc;

#[async_trait::async_trait]
impl RpcType for ReceiptRpc {
    type RpcReference = ReceiptRef;
    type Response = RpcReceiptResponse;
    type Error = RpcReceiptError;

    async fn send_query(
        &self,
        client: &Client,
        _network: &NetworkConfig,
        reference: &ReceiptRef,
    ) -> RetryResponse<RpcReceiptResponse, SendRequestError<RpcReceiptError>> {
        let response = client
            .experimental_receipt(&JsonRpcRequestForExperimentalReceipt {
                id: "0".to_string(),
                jsonrpc: "2.0".to_string(),
                method: JsonRpcRequestForExperimentalReceiptMethod::ExperimentalReceipt,
                params: RpcReceiptRequest {
                    receipt_id: reference.receipt_id.into(),
                },
            })
            .await
            .map(|r| r.into_inner())
            .map_err(SendRequestError::from);

        match response {
            Ok(JsonRpcResponseForRpcReceiptResponseAndRpcReceiptError::Variant0 {
                result, ..
            }) => RetryResponse::Ok(result),
            Ok(JsonRpcResponseForRpcReceiptResponseAndRpcReceiptError::Variant1 {
                error, ..
            }) => {
                let error = SendRequestError::from(error);
                to_retry_error(error, is_critical_receipt_error)
            }
            Err(err) => to_retry_error(err, is_critical_receipt_error),
        }
    }
}

impl From<ErrorWrapperForRpcReceiptError> for SendRequestError<RpcReceiptError> {
    fn from(err: ErrorWrapperForRpcReceiptError) -> Self {
        match err {
            ErrorWrapperForRpcReceiptError::InternalError(internal_error) => {
                Self::InternalError(internal_error)
            }
            ErrorWrapperForRpcReceiptError::RequestValidationError(
                rpc_request_validation_error_kind,
            ) => Self::RequestValidationError(rpc_request_validation_error_kind),
            ErrorWrapperForRpcReceiptError::HandlerError(server_error) => {
                Self::ServerError(server_error)
            }
        }
    }
}

/// Reference type for transaction proof queries.
///
/// Identifies a transaction proof by the sender, transaction hash, and the light client head block hash.
#[derive(Clone, Debug)]
pub struct TransactionProofRef {
    pub sender_id: AccountId,
    pub transaction_hash: CryptoHash,
    pub light_client_head: CryptoHash,
}

/// RPC type for fetching a light client execution proof for a transaction.
///
/// Uses the `light_client_proof` RPC method to retrieve the proof needed to verify
/// a transaction execution against a light client block.
#[derive(Clone, Debug)]
pub struct TransactionProofRpc;

#[async_trait::async_trait]
impl RpcType for TransactionProofRpc {
    type RpcReference = TransactionProofRef;
    type Response = RpcLightClientExecutionProofResponse;
    type Error = RpcLightClientProofError;

    async fn send_query(
        &self,
        client: &Client,
        _network: &NetworkConfig,
        reference: &TransactionProofRef,
    ) -> RetryResponse<
        RpcLightClientExecutionProofResponse,
        SendRequestError<RpcLightClientProofError>,
    > {
        let response = client
            .light_client_proof(&JsonRpcRequestForLightClientProof {
                id: "0".to_string(),
                jsonrpc: "2.0".to_string(),
                method: JsonRpcRequestForLightClientProofMethod::LightClientProof,
                params: RpcLightClientExecutionProofRequest::Variant0 {
                    sender_id: reference.sender_id.clone(),
                    transaction_hash: reference.transaction_hash.into(),
                    light_client_head: reference.light_client_head.into(),
                    type_: RpcLightClientExecutionProofRequestVariant0Type::Transaction,
                },
            })
            .await
            .map(|r| r.into_inner())
            .map_err(SendRequestError::from);

        match response {
            Ok(
                JsonRpcResponseForRpcLightClientExecutionProofResponseAndRpcLightClientProofError::Variant0 {
                    result,
                    ..
                },
            ) => RetryResponse::Ok(result),
            Ok(
                JsonRpcResponseForRpcLightClientExecutionProofResponseAndRpcLightClientProofError::Variant1 {
                    error,
                    ..
                },
            ) => {
                let error = SendRequestError::from(error);
                to_retry_error(error, is_critical_light_client_proof_error)
            }
            Err(err) => to_retry_error(err, is_critical_light_client_proof_error),
        }
    }
}

impl From<ErrorWrapperForRpcLightClientProofError> for SendRequestError<RpcLightClientProofError> {
    fn from(err: ErrorWrapperForRpcLightClientProofError) -> Self {
        match err {
            ErrorWrapperForRpcLightClientProofError::InternalError(internal_error) => {
                Self::InternalError(internal_error)
            }
            ErrorWrapperForRpcLightClientProofError::RequestValidationError(
                rpc_request_validation_error_kind,
            ) => Self::RequestValidationError(rpc_request_validation_error_kind),
            ErrorWrapperForRpcLightClientProofError::HandlerError(server_error) => {
                Self::ServerError(server_error)
            }
        }
    }
}
