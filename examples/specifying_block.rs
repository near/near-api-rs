use near_api::{prelude::*, types::reference::Reference};

#[tokio::main]
async fn main() {
    // Query latest block
    let block = Chain::block()
        .at(Reference::Optimistic)
        .fetch_from_mainnet()
        .await
        .unwrap();

    let block_number = Chain::block_number().fetch_from_mainnet().await.unwrap();

    let block1 = Chain::block()
        .at(Reference::AtBlock(block_number))
        .fetch_from_mainnet()
        .await
        .unwrap();

    let block = Chain::block()
        .at(Reference::AtBlockHash(block1.header.hash))
        .fetch_from_mainnet()
        .await
        .unwrap();

    assert_eq!(block, block1);
}
