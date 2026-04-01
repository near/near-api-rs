use near_api_types::EpochReference;
use near_openrpc_client::{BlockId, EpochId, RpcValidatorRequest, RpcValidatorResponse};

use crate::common::utils::to_retry_error;
use crate::errors::SendRequestError;
use crate::{
    NetworkConfig, advanced::RpcType, common::utils::is_critical_rpc_error, config::RetryResponse,
    rpc_client::RpcClient,
};

#[derive(Clone, Debug)]
pub struct SimpleValidatorRpc;

#[async_trait::async_trait]
impl RpcType for SimpleValidatorRpc {
    type RpcReference = EpochReference;
    type Response = RpcValidatorResponse;
    async fn send_query(
        &self,
        client: &RpcClient,
        _network: &NetworkConfig,
        reference: &EpochReference,
    ) -> RetryResponse<RpcValidatorResponse, SendRequestError> {
        let request = match reference {
            EpochReference::Latest => RpcValidatorRequest::Latest,
            EpochReference::AtEpoch(epoch) => {
                RpcValidatorRequest::EpochId(EpochId((*epoch).into()))
            }
            EpochReference::AtBlock(block) => {
                RpcValidatorRequest::BlockId(BlockId::BlockHeight(*block))
            }
            EpochReference::AtBlockHash(block_hash) => {
                RpcValidatorRequest::BlockId(BlockId::CryptoHash((*block_hash).into()))
            }
        };
        match client
            .call::<_, RpcValidatorResponse>("validators", request)
            .await
        {
            Ok(response) => RetryResponse::Ok(response),
            Err(err) => {
                let err = SendRequestError::from(err);
                to_retry_error(err, is_critical_rpc_error)
            }
        }
    }
}
