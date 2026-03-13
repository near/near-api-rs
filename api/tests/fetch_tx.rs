use near_api::{advanced::ExecuteSignedTransaction, *};
use near_api_types::{AccountId, NearToken};
use near_openapi_client::types::RpcTransactionStatusRequest;
use near_sandbox::config::{DEFAULT_GENESIS_ACCOUNT, DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY};
use testresult::TestResult;

#[tokio::test]
async fn fetch_tx_status() -> TestResult {
    let receiver: AccountId = "tmp_account".parse()?;
    let account: AccountId = DEFAULT_GENESIS_ACCOUNT.into();

    let sandbox = near_sandbox::Sandbox::start_sandbox().await?;
    sandbox.create_account(receiver.clone()).send().await?;

    let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse()?);
    let signer = Signer::from_secret_key(DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY.parse()?)?;

    let tx = Tokens::account(account.clone())
        .send_to(receiver.clone())
        .near(NearToken::from_millinear(1))
        .with_signer(signer.clone())
        .presign_with(&network)
        .await?;

    let tx_hash = tx.get_hash().unwrap();

    tx.wait_until(near_api_types::TxExecutionStatus::Included)
        .send_to(&network)
        .await?
        .assert_success();

    let res = ExecuteSignedTransaction::fetch_tx(
        &network,
        RpcTransactionStatusRequest::Variant1 {
            sender_account_id: account.clone(),
            tx_hash: tx_hash.into(),
            wait_until: near_api_types::TxExecutionStatus::IncludedFinal,
        },
    )
    .await?
    .assert_success();

    assert!(res.outcome().is_success());

    Ok(())
}
