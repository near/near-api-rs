use anyhow::bail;
use near_crypto::PublicKey;
use near_primitives::{
    action::delegate::SignedDelegateAction,
    hash::CryptoHash,
    transaction::SignedTransaction,
    types::{BlockHeight, Nonce},
    views::FinalExecutionOutcomeView,
};
use near_token::NearToken;

use crate::{config::NetworkConfig, sign::SignerTrait, transactions::PrepopulateTransaction};

use super::{
    signed_delegate_action::SignedDelegateActionAsBase64, META_TRANSACTION_VALID_FOR_DEFAULT,
};

pub trait Transactionable {
    fn prepopulated(&self) -> PrepopulateTransaction;
    fn validate_with_network(
        tx: &PrepopulateTransaction,
        network: &NetworkConfig,
    ) -> anyhow::Result<()>;
}

pub enum TransactionableOrSigned<T, Signed> {
    Prepopulated(T),
    Signed((Signed, T)),
}

impl<T, Signed> TransactionableOrSigned<T, Signed> {
    pub fn signed(self) -> Option<Signed> {
        match self {
            TransactionableOrSigned::Signed((signed, _)) => Some(signed),
            TransactionableOrSigned::Prepopulated(_) => None,
        }
    }
}

impl<T, S> TransactionableOrSigned<T, S> {
    pub fn transactionable(self) -> T {
        match self {
            TransactionableOrSigned::Prepopulated(tr) => tr,
            TransactionableOrSigned::Signed((_, tr)) => tr,
        }
    }
}

impl From<SignedTransaction> for PrepopulateTransaction {
    fn from(tr: SignedTransaction) -> Self {
        PrepopulateTransaction {
            signer_id: tr.transaction.signer_id,
            receiver_id: tr.transaction.receiver_id,
            actions: tr.transaction.actions,
        }
    }
}

pub struct ExecuteSignedTransaction<T: Transactionable> {
    pub tr: TransactionableOrSigned<T, SignedTransaction>,
    pub signer: Box<dyn SignerTrait>,
}

impl<T: Transactionable> ExecuteSignedTransaction<T> {
    pub fn new(tr: T, signer: Box<dyn SignerTrait>) -> Self {
        Self {
            tr: TransactionableOrSigned::Prepopulated(tr),
            signer,
        }
    }

    pub fn meta(self) -> ExecuteMetaTransaction<T> {
        ExecuteMetaTransaction::new(self.tr.transactionable(), self.signer)
    }

    pub fn presign_offline(
        mut self,
        public_key: PublicKey,
        block_hash: CryptoHash,
        nonce: Nonce,
    ) -> anyhow::Result<Self> {
        let tr = match &self.tr {
            TransactionableOrSigned::Prepopulated(tr) => tr,
            TransactionableOrSigned::Signed(_) => return Ok(self),
        };
        let signed_tr = self
            .signer
            .sign(tr.prepopulated(), public_key, nonce, block_hash)?;
        self.tr = TransactionableOrSigned::Signed((signed_tr, self.tr.transactionable()));
        Ok(self)
    }

    pub async fn presign_with(self, network: &NetworkConfig) -> anyhow::Result<Self> {
        let tr = match &self.tr {
            TransactionableOrSigned::Prepopulated(tr) => tr,
            TransactionableOrSigned::Signed(_) => return Ok(self),
        };

        let signer_key = self.signer.get_public_key()?;
        let tr = tr.prepopulated();
        let response = crate::account::Account(tr.signer_id.clone())
            .access_key(signer_key.clone())
            .fetch_from(network)
            .await?;
        self.presign_offline(signer_key, response.block_hash, response.data.nonce + 1)
    }

    pub async fn presign_with_mainnet(self) -> anyhow::Result<Self> {
        let network = NetworkConfig::mainnet();
        self.presign_with(&network).await
    }

    pub async fn presign_with_testnet(self) -> anyhow::Result<Self> {
        let network = NetworkConfig::testnet();
        self.presign_with(&network).await
    }

    pub async fn send_to(
        self,
        network: &NetworkConfig,
    ) -> anyhow::Result<FinalExecutionOutcomeView> {
        let (signed, tr) = match self.tr {
            TransactionableOrSigned::Prepopulated(_) => {
                match self.presign_with(network).await?.tr {
                    TransactionableOrSigned::Signed((s, tr)) => (s, tr),
                    TransactionableOrSigned::Prepopulated(_) => unreachable!(),
                }
            }
            TransactionableOrSigned::Signed((s, tr)) => (s, tr),
        };
        T::validate_with_network(&tr.prepopulated(), network)?;

        Self::send_impl(network, signed).await
    }

    pub async fn send_to_mainnet(self) -> anyhow::Result<FinalExecutionOutcomeView> {
        let network = NetworkConfig::mainnet();
        self.send_to(&network).await
    }

    pub async fn send_to_testnet(self) -> anyhow::Result<FinalExecutionOutcomeView> {
        let network = NetworkConfig::testnet();
        self.send_to(&network).await
    }

    // TODO: More configurable timeouts and retry policy
    async fn send_impl(
        network: &NetworkConfig,
        signed_tr: SignedTransaction,
    ) -> anyhow::Result<FinalExecutionOutcomeView> {
        let retries_number = 5;
        let mut retries = (1..=retries_number).rev();
        let transaction_info = loop {
            let transaction_info_result = network
                .json_rpc_client()
                .call(
                    near_jsonrpc_client::methods::broadcast_tx_commit::RpcBroadcastTxCommitRequest {
                        signed_transaction: signed_tr.clone(),
                    },
                )
                .await;
            match transaction_info_result {
                Ok(response) => {
                    break response;
                }
                Err(ref err) => match rpc_transaction_error(err) {
                    Ok(_) => {
                        if retries.next().is_some() {
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

pub struct ExecuteMetaTransaction<T> {
    pub tr: TransactionableOrSigned<T, SignedDelegateAction>,
    pub signer: Box<dyn SignerTrait>,
    pub tx_live_for: Option<BlockHeight>,
}

impl<T: Transactionable> ExecuteMetaTransaction<T> {
    pub fn new(tr: T, signer: Box<dyn SignerTrait>) -> Self {
        Self {
            tr: TransactionableOrSigned::Prepopulated(tr),
            signer,
            tx_live_for: None,
        }
    }

    pub fn tx_live_for(mut self, tx_live_for: BlockHeight) -> Self {
        self.tx_live_for = Some(tx_live_for);
        self
    }

    pub fn presign_offline(
        mut self,
        public_key: PublicKey,
        block_hash: CryptoHash,
        nonce: Nonce,
        block_height: BlockHeight,
    ) -> anyhow::Result<Self> {
        let tr = match &self.tr {
            TransactionableOrSigned::Prepopulated(tr) => tr,
            TransactionableOrSigned::Signed(_) => return Ok(self),
        };

        let max_block_height = block_height
            + self
                .tx_live_for
                .unwrap_or(META_TRANSACTION_VALID_FOR_DEFAULT);
        let signed_tr = self.signer.sign_meta(
            tr.prepopulated(),
            public_key,
            nonce,
            block_hash,
            max_block_height,
        )?;
        self.tr = TransactionableOrSigned::Signed((signed_tr, self.tr.transactionable()));
        Ok(self)
    }

    pub async fn presign_with(self, network: &NetworkConfig) -> anyhow::Result<Self> {
        let tr = match &self.tr {
            TransactionableOrSigned::Prepopulated(tr) => tr,
            TransactionableOrSigned::Signed(_) => return Ok(self),
        };

        let signer_key = self.signer.get_public_key()?;
        let response = crate::account::Account(tr.prepopulated().signer_id.clone())
            .access_key(signer_key.clone())
            .fetch_from(network)
            .await?;
        self.presign_offline(
            signer_key,
            response.block_hash,
            response.data.nonce + 1,
            response.block_height,
        )
    }

    pub async fn presign_with_mainnet(self) -> anyhow::Result<Self> {
        let network = NetworkConfig::mainnet();
        self.presign_with(&network).await
    }

    pub async fn presign_with_testnet(self) -> anyhow::Result<Self> {
        let network = NetworkConfig::testnet();
        self.presign_with(&network).await
    }

    pub async fn send_to(self, network: &NetworkConfig) -> anyhow::Result<reqwest::Response> {
        let (signed, tr) = match self.tr {
            TransactionableOrSigned::Prepopulated(_) => {
                match self.presign_with(network).await?.tr {
                    TransactionableOrSigned::Signed((s, tr)) => (s, tr),
                    TransactionableOrSigned::Prepopulated(_) => unreachable!(),
                }
            }
            TransactionableOrSigned::Signed((s, tr)) => (s, tr),
        };
        T::validate_with_network(&tr.prepopulated(), network)?;
        let transaction_info = Self::send_impl(network, signed).await?;
        Ok(transaction_info)
    }

    pub async fn send_to_mainnet(self) -> anyhow::Result<reqwest::Response> {
        let network = NetworkConfig::mainnet();
        self.send_to(&network).await
    }

    pub async fn send_to_testnet(self) -> anyhow::Result<reqwest::Response> {
        let network = NetworkConfig::testnet();
        self.send_to(&network).await
    }

    async fn send_impl(
        network: &NetworkConfig,
        tr: SignedDelegateAction,
    ) -> anyhow::Result<reqwest::Response> {
        let client = reqwest::Client::new();
        let json_payload = serde_json::json!({
            "signed_delegate_action": SignedDelegateActionAsBase64::from(
                tr
            ).to_string(),
        });
        let resp = client
            .post(
                network
                    .meta_transaction_relayer_url
                    .clone()
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
