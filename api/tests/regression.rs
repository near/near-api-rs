#[tokio::test]
/// Regression test for https://github.com/near/near-api-rs/issues/85
async fn regression_85() {
    let sandbox = near_sandbox::Sandbox::start_sandbox().await.unwrap();
    let network_config =
        near_api::NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse().unwrap());

    let contract = near_api::Contract(near_sandbox::config::DEFAULT_GENESIS_ACCOUNT.into())
        .call_function("increment", ())
        .unwrap()
        .read_only::<u64>()
        .fetch_from(&network_config)
        .await
        .expect_err("Should fail as the contract is not deployed");

    // Should be a WASM execution error rather than TransportError(Invalid Response Payload
    let query_error = match contract {
        near_api::errors::QueryError::QueryError(query) => query,
        _ => panic!("Should be a QueryError"),
    };

    let retry_error = match *query_error {
        near_api::errors::RetryError::Critical(boxed_err) => boxed_err,
        _ => panic!("Should be a RetryError"),
    };

    assert!(matches!(
        retry_error,
        near_api::errors::SendRequestError::WasmExecutionError(_)
    ));
}
