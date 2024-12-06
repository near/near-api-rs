use near_api::{prelude::*, types::reference::Reference};

#[tokio::main]
async fn main() {
    let mut network = NetworkConfig::mainnet();
    network.rpc_endpoints.push(
        RPCEndpoint::new("https://rpc.mainnet.pagoda.co/".parse().unwrap())
            .with_api_key("potential api key".parse().unwrap())
            .with_retries(5),
    );
    // Query latest block
    let _block = Chain::block()
        .at(Reference::Optimistic)
        .fetch_from_mainnet()
        .await
        .unwrap();
}
