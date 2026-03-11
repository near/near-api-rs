use near_api_types::Reference;
use near_openrpc_client::{BlockId, Finality, RpcBlockRequest, RpcBlockResponse};

use crate::common::utils::to_retry_error;
use crate::{
    NetworkConfig, advanced::RpcType, common::utils::is_critical_rpc_error, config::RetryResponse,
    errors::SendRequestError, rpc_client::RpcClient,
};

#[derive(Clone, Debug)]
pub struct SimpleBlockRpc;

#[async_trait::async_trait]
impl RpcType for SimpleBlockRpc {
    type RpcReference = Reference;
    type Response = RpcBlockResponse;
    async fn send_query(
        &self,
        client: &RpcClient,
        _network: &NetworkConfig,
        reference: &Reference,
    ) -> RetryResponse<RpcBlockResponse, SendRequestError> {
        let request = match reference {
            Reference::Optimistic => RpcBlockRequest::Finality(Finality::Optimistic),
            Reference::NearFinal => RpcBlockRequest::Finality(Finality::NearFinal),
            Reference::Final => RpcBlockRequest::Finality(Finality::Final),
            Reference::AtBlock(block) => RpcBlockRequest::BlockId(BlockId::BlockHeight(*block)),
            Reference::AtBlockHash(block_hash) => {
                RpcBlockRequest::BlockId(BlockId::CryptoHash((*block_hash).into()))
            }
        };
        match client.call::<_, RpcBlockResponse>("block", request).await {
            Ok(response) => RetryResponse::Ok(response),
            Err(err) => {
                let err = SendRequestError::from(err);
                to_retry_error(err, is_critical_rpc_error)
            }
        }
    }
}
