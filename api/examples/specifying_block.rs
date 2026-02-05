use near_api::{Chain, types::Reference};

#[tokio::main]
async fn main() -> testresult::TestResult {
    // Fetch a single final block
    let final_block = Chain::block()
        .at(Reference::Final)
        .fetch_from_mainnet()
        .await?;

    let block_number = final_block.header.height;
    let block_hash = final_block.header.hash.into();

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
