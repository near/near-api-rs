use near_api::{prelude::*, types::reference::Reference};

#[tokio::main]
async fn main() {
    let metadata = Contract("race-of-sloths.near".parse().unwrap())
        .contract_source_metadata()
        .at(Reference::Optimistic)
        .fetch_from_mainnet()
        .await
        .unwrap();

    println!("{:?}", metadata);
}
