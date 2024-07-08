use anyhow::bail;
use near_primitives::{
    action::delegate::SignedDelegateAction, transaction::SignedTransaction,
    views::FinalExecutionOutcomeView,
};
use near_token::NearToken;

use crate::{config::NetworkConfig, signed_delegate_action::SignedDelegateActionAsBase64};

pub struct SendSignedTransaction {
    pub signed_transaction: SignedTransaction,
    pub network: NetworkConfig,
}

impl SendSignedTransaction {
    pub async fn send(self) -> anyhow::Result<FinalExecutionOutcomeView> {
        let retries_number = 5;
        let mut retries = (1..=retries_number).rev();
        let transaction_info = loop {
            let transaction_info_result = self.network
                .json_rpc_client()
                .call(
                    near_jsonrpc_client::methods::broadcast_tx_commit::RpcBroadcastTxCommitRequest {
                        signed_transaction: self.signed_transaction.clone(),
                    },
                )
                .await;
            match transaction_info_result {
                Ok(response) => {
                    break response;
                }
                Err(ref err) => match rpc_transaction_error(err) {
                    Ok(_) => {
                        if let Some(_) = retries.next() {
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        } else {
                            bail!(err.to_string());
                        }
                    }
                    Err(report) => bail!(report.to_string()),
                },
            };
        };
        Ok(transaction_info)
    }
}

pub struct SendMetaTransaction {
    pub signed_delegate_action: SignedDelegateAction,
    pub network: NetworkConfig,
}

impl SendMetaTransaction {
    pub async fn send(self) -> anyhow::Result<reqwest::Response> {
        let client = reqwest::Client::new();
        let json_payload = serde_json::json!({
            "signed_delegate_action": SignedDelegateActionAsBase64::from(
                self.signed_delegate_action
            ).to_string(),
        });
        let resp = client
            .post(
                self.network
                    .meta_transaction_relayer_url
                    .ok_or_else(|| anyhow::anyhow!("Meta transaction relayer URL is not set"))?,
            )
            .json(&json_payload)
            .send()
            .await?;
        Ok(resp)
    }
}

pub fn rpc_transaction_error(
    err: &near_jsonrpc_client::errors::JsonRpcError<
        near_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError,
    >,
) -> anyhow::Result<String> {
    match &err {
        near_jsonrpc_client::errors::JsonRpcError::TransportError(_rpc_transport_error) => {
            Ok("Transport error transaction".to_string())
        }
        near_jsonrpc_client::errors::JsonRpcError::ServerError(rpc_server_error) => match rpc_server_error {
            near_jsonrpc_client::errors::JsonRpcServerError::HandlerError(rpc_transaction_error) => match rpc_transaction_error {
                near_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::TimeoutError => {
                    Ok("Timeout error transaction".to_string())
                }
                near_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::InvalidTransaction { context } => {
                    match convert_invalid_tx_error_to_cli_result(context) {
                        Ok(_) => Ok("".to_string()),
                        Err(err) => Err(err)
                    }
                }
                near_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::DoesNotTrackShard => {
                    anyhow::Result::Err(anyhow::anyhow!("RPC Server Error: {}", err))
                }
                near_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::RequestRouted{transaction_hash} => {
                    anyhow::Result::Err(anyhow::anyhow!("RPC Server Error for transaction with hash {}\n{}", transaction_hash, err))
                }
                near_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::UnknownTransaction{requested_transaction_hash} => {
                    anyhow::Result::Err(anyhow::anyhow!("RPC Server Error for transaction with hash {}\n{}", requested_transaction_hash, err))
                }
                near_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::InternalError{debug_info} => {
                    anyhow::Result::Err(anyhow::anyhow!("RPC Server Error: {}", debug_info))
                }
            }
            near_jsonrpc_client::errors::JsonRpcServerError::RequestValidationError(rpc_request_validation_error) => {
                anyhow::Result::Err(anyhow::anyhow!("Incompatible request with the server: {:#?}",  rpc_request_validation_error))
            }
            near_jsonrpc_client::errors::JsonRpcServerError::InternalError{ info } => {
                Ok(format!("Internal server error: {}", info.clone().unwrap_or_default()))
            }
            near_jsonrpc_client::errors::JsonRpcServerError::NonContextualError(rpc_error) => {
                anyhow::Result::Err(anyhow::anyhow!("Unexpected response: {}", rpc_error))
            }
            near_jsonrpc_client::errors::JsonRpcServerError::ResponseStatusError(json_rpc_server_response_status_error) => match json_rpc_server_response_status_error {
                near_jsonrpc_client::errors::JsonRpcServerResponseStatusError::Unauthorized => {
                    anyhow::Result::Err(anyhow::anyhow!("JSON RPC server requires authentication. Please, authenticate near CLI with the JSON RPC server you use."))
                }
                near_jsonrpc_client::errors::JsonRpcServerResponseStatusError::TooManyRequests => {
                    Ok("JSON RPC server is currently busy".to_string())
                }
                near_jsonrpc_client::errors::JsonRpcServerResponseStatusError::Unexpected{status} => {
                    anyhow::Result::Err(anyhow::anyhow!("JSON RPC server responded with an unexpected status code: {}", status))
                }
            }
        }
    }
}

pub fn convert_invalid_tx_error_to_cli_result(
    invalid_tx_error: &near_primitives::errors::InvalidTxError,
) -> anyhow::Result<()> {
    match invalid_tx_error {
        near_primitives::errors::InvalidTxError::InvalidAccessKeyError(invalid_access_key_error) => {
            match invalid_access_key_error {
                near_primitives::errors::InvalidAccessKeyError::AccessKeyNotFound{account_id, public_key} => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: Public key {} doesn't exist for the account <{}>.", public_key, account_id))
                },
                near_primitives::errors::InvalidAccessKeyError::ReceiverMismatch{tx_receiver, ak_receiver} => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: Transaction for <{}> doesn't match the access key for <{}>.", tx_receiver, ak_receiver))
                },
                near_primitives::errors::InvalidAccessKeyError::MethodNameMismatch{method_name} => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: Transaction method name <{}> isn't allowed by the access key.", method_name))
                },
                near_primitives::errors::InvalidAccessKeyError::RequiresFullAccess => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: Transaction requires a full permission access key."))
                },
                near_primitives::errors::InvalidAccessKeyError::NotEnoughAllowance{account_id, public_key, allowance, cost} => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: Access Key <{}> for account <{}> does not have enough allowance ({}) to cover transaction cost ({}).",
                        public_key,
                        account_id,
                        NearToken::from_yoctonear(*allowance),
                        NearToken::from_yoctonear(*cost)
                    ))
                },
                near_primitives::errors::InvalidAccessKeyError::DepositWithFunctionCall => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: Having a deposit with a function call action is not allowed with a function call access key."))
                }
            }
        },
        near_primitives::errors::InvalidTxError::InvalidSignerId { signer_id } => {
            anyhow::Result::Err(anyhow::anyhow!("Error: TX signer ID <{}> is not in a valid format or does not satisfy requirements\nSee \"near_runtime_utils::utils::is_valid_account_id\".", signer_id))
        },
        near_primitives::errors::InvalidTxError::SignerDoesNotExist { signer_id } => {
            anyhow::Result::Err(anyhow::anyhow!("Error: TX signer ID <{}> is not found in the storage.", signer_id))
        },
        near_primitives::errors::InvalidTxError::InvalidNonce { tx_nonce, ak_nonce } => {
            anyhow::Result::Err(anyhow::anyhow!("Error: Transaction nonce ({}) must be account[access_key].nonce ({}) + 1.", tx_nonce, ak_nonce))
        },
        near_primitives::errors::InvalidTxError::NonceTooLarge { tx_nonce, upper_bound } => {
            anyhow::Result::Err(anyhow::anyhow!("Error: Transaction nonce ({}) is larger than the upper bound ({}) given by the block height.", tx_nonce, upper_bound))
        },
        near_primitives::errors::InvalidTxError::InvalidReceiverId { receiver_id } => {
            anyhow::Result::Err(anyhow::anyhow!("Error: TX receiver ID ({}) is not in a valid format or does not satisfy requirements\nSee \"near_runtime_utils::is_valid_account_id\".", receiver_id))
        },
        near_primitives::errors::InvalidTxError::InvalidSignature => {
            anyhow::Result::Err(anyhow::anyhow!("Error: TX signature is not valid"))
        },
        near_primitives::errors::InvalidTxError::NotEnoughBalance {signer_id, balance, cost} => {
            anyhow::Result::Err(anyhow::anyhow!("Error: Account <{}> does not have enough balance ({}) to cover TX cost ({}).",
                signer_id,
                NearToken::from_yoctonear(*balance),
                NearToken::from_yoctonear(*cost)
            ))
        },
        near_primitives::errors::InvalidTxError::LackBalanceForState {signer_id, amount} => {
            anyhow::Result::Err(anyhow::anyhow!("Error: Signer account <{}> doesn't have enough balance ({}) after transaction.",
                signer_id,
                NearToken::from_yoctonear(*amount)
            ))
        },
        near_primitives::errors::InvalidTxError::CostOverflow => {
            anyhow::Result::Err(anyhow::anyhow!("Error: An integer overflow occurred during transaction cost estimation."))
        },
        near_primitives::errors::InvalidTxError::InvalidChain => {
            anyhow::Result::Err(anyhow::anyhow!("Error: Transaction parent block hash doesn't belong to the current chain."))
        },
        near_primitives::errors::InvalidTxError::Expired => {
            anyhow::Result::Err(anyhow::anyhow!("Error: Transaction has expired."))
        },
        near_primitives::errors::InvalidTxError::ActionsValidation(actions_validation_error) => {
            match actions_validation_error {
                near_primitives::errors::ActionsValidationError::DeleteActionMustBeFinal => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: The delete action must be the final action in transaction."))
                },
                near_primitives::errors::ActionsValidationError::TotalPrepaidGasExceeded {total_prepaid_gas, limit} => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: The total prepaid gas ({}) for all given actions exceeded the limit ({}).",
                    total_prepaid_gas,
                    limit
                    ))
                },
                near_primitives::errors::ActionsValidationError::TotalNumberOfActionsExceeded {total_number_of_actions, limit} => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: The number of actions ({}) exceeded the given limit ({}).", total_number_of_actions, limit))
                },
                near_primitives::errors::ActionsValidationError::AddKeyMethodNamesNumberOfBytesExceeded {total_number_of_bytes, limit} => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: The total number of bytes ({}) of the method names exceeded the limit ({}) in a Add Key action.", total_number_of_bytes, limit))
                },
                near_primitives::errors::ActionsValidationError::AddKeyMethodNameLengthExceeded {length, limit} => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: The length ({}) of some method name exceeded the limit ({}) in a Add Key action.", length, limit))
                },
                near_primitives::errors::ActionsValidationError::IntegerOverflow => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: Integer overflow."))
                },
                near_primitives::errors::ActionsValidationError::InvalidAccountId {account_id} => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: Invalid account ID <{}>.", account_id))
                },
                near_primitives::errors::ActionsValidationError::ContractSizeExceeded {size, limit} => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: The size ({}) of the contract code exceeded the limit ({}) in a DeployContract action.", size, limit))
                },
                near_primitives::errors::ActionsValidationError::FunctionCallMethodNameLengthExceeded {length, limit} => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: The length ({}) of the method name exceeded the limit ({}) in a Function Call action.", length, limit))
                },
                near_primitives::errors::ActionsValidationError::FunctionCallArgumentsLengthExceeded {length, limit} => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: The length ({}) of the arguments exceeded the limit ({}) in a Function Call action.", length, limit))
                },
                near_primitives::errors::ActionsValidationError::UnsuitableStakingKey {public_key} => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: An attempt to stake with a public key <{}> that is not convertible to ristretto.", public_key))
                },
                near_primitives::errors::ActionsValidationError::FunctionCallZeroAttachedGas => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: The attached amount of gas in a FunctionCall action has to be a positive number."))
                }
                near_primitives::errors::ActionsValidationError::DelegateActionMustBeOnlyOne => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: DelegateActionMustBeOnlyOne"))
                }
                near_primitives::errors::ActionsValidationError::UnsupportedProtocolFeature { protocol_feature, version } => {
                    anyhow::Result::Err(anyhow::anyhow!("Error: Protocol Feature {} is unsupported in version {}", protocol_feature, version))
                }
            }
        },
        near_primitives::errors::InvalidTxError::TransactionSizeExceeded { size, limit } => {
            anyhow::Result::Err(anyhow::anyhow!("Error: The size ({}) of serialized transaction exceeded the limit ({}).", size, limit))
        }
    }
}
