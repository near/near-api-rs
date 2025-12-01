use near_api::{types::Reference, Chain};

#[tokio::main]
async fn main() -> testresult::TestResult {
    // Query latest block
    let _block = Chain::block()
        .at(Reference::Optimistic)
        .fetch_from_mainnet()
        .await?;

    let block_number = Chain::block_number().fetch_from_mainnet().await?;
    let block_hash = Chain::block_hash().fetch_from_mainnet().await?;

    let _block = Chain::block()
        .at(Reference::AtBlock(block_number))
        .fetch_from_mainnet()
        .await?;

    let _block = Chain::block()
        .at(Reference::AtBlockHash(block_hash))
        .fetch_from_mainnet()
        .await?;

    Ok(())
}
