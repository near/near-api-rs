use near_api::{
    Chain, Transaction,
    types::{AccountId, CryptoHash, Reference, TxExecutionStatus},
};
use testresult::TestResult;

/// This example queries mainnet with a known finalized transaction.
///
/// Sandbox does not support `EXPERIMENTAL_receipt` or `light_client_proof`,
/// so this example is skipped when `CI=true`.
///
/// The transaction hash and sender can be overridden via environment variables:
///   TX_HASH  — the transaction hash to query
///   TX_SENDER — the sender account ID
#[tokio::main]
async fn main() -> TestResult {
    if std::env::var("CI").is_ok() {
        println!("Skipping transaction_queries in CI (requires mainnet)");
        return Ok(());
    }

    let sender: AccountId = std::env::var("TX_SENDER")
        .unwrap_or_else(|_| "omni.bridge.near".to_string())
        .parse()?;
    let tx_hash: CryptoHash = std::env::var("TX_HASH")
        .unwrap_or_else(|_| "GmvjRhbBwNCeekyZ4ezv43Zhs4U33kRTj6PRkFgKUKyJ".to_string())
        .parse()?;

    let status = Transaction::status(sender.clone(), tx_hash)
        .fetch_from_mainnet_archival()
        .await?;
    println!(
        "[status] is_success={}, is_failure={}, gas_burnt={}",
        status.is_success(),
        status.is_failure(),
        status.total_gas_burnt,
    );

    let status_final =
        Transaction::status_with_options(sender.clone(), tx_hash, TxExecutionStatus::Final)
            .fetch_from_mainnet_archival()
            .await?;
    println!(
        "[status_with_options(Final)] is_success={}, receipts={}",
        status_final.is_success(),
        status_final.receipt_outcomes().len(),
    );

    let receipt_id = *status_final
        .outcome()
        .receipt_ids
        .first()
        .expect("transaction should have at least one receipt");
    let receipt = Transaction::receipt(receipt_id)
        .fetch_from_mainnet_archival()
        .await?;
    println!(
        "[receipt] id={}, receiver={}, predecessor={}",
        receipt.receipt_id, receipt.receiver_id, receipt.predecessor_id,
    );

    let head_hash = Chain::block_hash()
        .at(Reference::Final)
        .fetch_from_mainnet_archival()
        .await?;
    let proof = Transaction::proof(sender, tx_hash, head_hash)
        .fetch_from_mainnet_archival()
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
