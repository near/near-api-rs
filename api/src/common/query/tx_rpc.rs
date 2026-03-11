use near_api_types::{AccountId, CryptoHash, RpcReceiptResponse, TxExecutionStatus};
use near_openrpc_client::{
    RpcLightClientExecutionProofRequest, RpcLightClientExecutionProofResponse, RpcReceiptRequest,
    RpcTransactionResponse, RpcTransactionStatusRequest,
};

use crate::common::utils::to_retry_error;
use crate::{
    NetworkConfig,
    advanced::RpcType,
    common::utils::{
        is_critical_light_client_proof_error, is_critical_receipt_error,
        is_critical_transaction_status_error,
    },
    config::RetryResponse,
    errors::SendRequestError,
    rpc_client::RpcClient,
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
    type Response = RpcTransactionResponse;

    async fn send_query(
        &self,
        client: &RpcClient,
        _network: &NetworkConfig,
        reference: &TransactionStatusRef,
    ) -> RetryResponse<Self::Response, SendRequestError> {
        let request = RpcTransactionStatusRequest::TxHashSenderAccountId {
            sender_account_id: near_openrpc_client::AccountId(reference.sender_account_id.to_string()),
            tx_hash: reference.tx_hash.into(),
            wait_until: reference.wait_until,
        };

        match client.call::<_, RpcTransactionResponse>("tx", request).await {
            Ok(response) => RetryResponse::Ok(response),
            Err(err) => {
                to_retry_error(SendRequestError::from(err), is_critical_transaction_status_error)
            }
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

    async fn send_query(
        &self,
        client: &RpcClient,
        _network: &NetworkConfig,
        reference: &ReceiptRef,
    ) -> RetryResponse<RpcReceiptResponse, SendRequestError> {
        let request = RpcReceiptRequest {
            receipt_id: reference.receipt_id.into(),
        };

        match client
            .call::<_, RpcReceiptResponse>("EXPERIMENTAL_receipt", request)
            .await
        {
            Ok(response) => RetryResponse::Ok(response),
            Err(err) => to_retry_error(SendRequestError::from(err), is_critical_receipt_error),
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

    async fn send_query(
        &self,
        client: &RpcClient,
        _network: &NetworkConfig,
        reference: &TransactionProofRef,
    ) -> RetryResponse<RpcLightClientExecutionProofResponse, SendRequestError> {
        let request = RpcLightClientExecutionProofRequest::Transaction {
            sender_id: near_openrpc_client::AccountId(reference.sender_id.to_string()),
            transaction_hash: reference.transaction_hash.into(),
            light_client_head: reference.light_client_head.into(),
        };

        match client
            .call::<_, RpcLightClientExecutionProofResponse>("light_client_proof", request)
            .await
        {
            Ok(response) => RetryResponse::Ok(response),
            Err(err) => {
                to_retry_error(
                    SendRequestError::from(err),
                    is_critical_light_client_proof_error,
                )
            }
        }
    }
}
