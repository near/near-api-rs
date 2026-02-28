use near_api::{
    Chain, Transaction,
    types::{AccountId, CryptoHash, Reference, TxExecutionStatus},
};
use testresult::TestResult;

#[tokio::main]
async fn main() -> TestResult {
    let sender: AccountId = "omni.bridge.near".parse()?;
    let tx_hash: CryptoHash = "GmvjRhbBwNCeekyZ4ezv43Zhs4U33kRTj6PRkFgKUKyJ".parse()?;

    let status = Transaction::status(sender.clone(), tx_hash)
        .fetch_from_mainnet()
        .await?;
    println!(
        "[status] is_success={}, is_failure={}, gas_burnt={}",
        status.is_success(),
        status.is_failure(),
        status.total_gas_burnt,
    );

    let status_final =
        Transaction::status_with_options(sender.clone(), tx_hash, TxExecutionStatus::Final)
            .fetch_from_mainnet()
            .await?;
    println!(
        "[status_with_options(Final)] is_success={}, receipts={}",
        status_final.is_success(),
        status_final.receipt_outcomes().len(),
    );

    let receipt_id = status_final.outcome().receipt_ids[0];
    let receipt = Transaction::receipt(receipt_id)
        .fetch_from_mainnet()
        .await?;
    println!(
        "[receipt] id={}, receiver={}, predecessor={}",
        receipt.receipt_id, receipt.receiver_id, receipt.predecessor_id,
    );

    let head_hash = Chain::block_hash()
        .at(Reference::Final)
        .fetch_from_mainnet()
        .await?;
    let proof = Transaction::proof(sender, tx_hash, head_hash)
        .fetch_from_mainnet()
        .await?;
    println!(
        "[proof] outcome_proof_id={}, block_proof_len={}, outcome_root_proof_len={}",
        proof.outcome_proof.id,
        proof.block_proof.len(),
        proof.outcome_root_proof.len(),
    );

    println!("\nAll transaction query methods passed!");

    Ok(())
}
