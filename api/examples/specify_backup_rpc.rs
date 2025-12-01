use near_api::{types::Reference, Chain, NetworkConfig, RPCEndpoint};

#[tokio::main]
async fn main() -> testresult::TestResult {
    let mut network = NetworkConfig::mainnet();
    network.rpc_endpoints.push(
        RPCEndpoint::new("https://near.lava.build:443".parse()?)
            .with_retries(5)
            .with_api_key("some potential api key".to_string()),
    );
    // Query latest block
    let _block = Chain::block()
        .at(Reference::Optimistic)
        .fetch_from_mainnet()
        .await?;

    Ok(())
}
