use near_api::{Chain, types::Reference};

#[tokio::main]
async fn main() -> testresult::TestResult {
    // Fetch a optimistic block
    let _optimistic_block = Chain::block().fetch_from_mainnet().await?;

    let block_number = Chain::block_number()
        .at(Reference::Final)
        .fetch_from_mainnet()
        .await?;

    let block_hash = Chain::block_hash()
        .at(Reference::Final)
        .fetch_from_mainnet()
        .await?;

    let _block_by_number = Chain::block()
        .at(Reference::AtBlock(block_number))
        .fetch_from_mainnet()
        .await?;

    let _block_by_hash = Chain::block()
        .at(Reference::AtBlockHash(block_hash))
        .fetch_from_mainnet()
        .await?;

    Ok(())
}
