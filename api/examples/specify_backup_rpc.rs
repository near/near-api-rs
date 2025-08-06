use near_api::{Chain, NetworkConfig, RPCEndpoint, types::Reference};

#[tokio::main]
async fn main() {
    let mut network = NetworkConfig::mainnet();
    network.rpc_endpoints.push(
        RPCEndpoint::new("https://near.lava.build:443".parse().unwrap())
            .with_retries(5)
            .with_api_key("some potential api key".to_string()),
    );
    // Query latest block
    let _block = Chain::block()
        .at(Reference::Optimistic)
        .fetch_from_mainnet()
        .await
        .unwrap();
}
