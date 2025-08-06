use near_api::{Chain, types::Reference};

#[tokio::main]
async fn main() {
    // Query latest block
    let _block = Chain::block()
        .at(Reference::Optimistic)
        .fetch_from_mainnet()
        .await
        .unwrap();

    let block_number = Chain::block_number().fetch_from_mainnet().await.unwrap();
    let block_hash = Chain::block_hash().fetch_from_mainnet().await.unwrap();

    let _block = Chain::block()
        .at(Reference::AtBlock(block_number))
        .fetch_from_mainnet()
        .await
        .unwrap();

    let _block = Chain::block()
        .at(Reference::AtBlockHash(block_hash))
        .fetch_from_mainnet()
        .await
        .unwrap();
}
