use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use jito_sdk_rust::JitoJsonRpcSDK;
use serde_json::json;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    signer::EncodableKey,
    system_instruction,
    transaction::Transaction,
};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up Solana RPC client (for getting recent blockhash and confirming transaction)
    let solana_rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());

    // Setup client Jito Block Engine endpoint
    let jito_sdk = JitoJsonRpcSDK::new("https://mainnet.block-engine.jito.wtf/api/v1", None);

    // Setup client Jito Block Engine endpoint with UUID
    //let jito_sdk = JitoJsonRpcSDK::new("https://mainnet.block-engine.jito.wtf/api/v1", "UUID-API-KEY");

    // Load the sender's keypair
    let sender =
        Keypair::read_from_file("/path/to/wallet.json").expect("Failed to read wallet file");

    println!("Sender pubkey: {}", sender.pubkey());

    // Set up receiver and Jito tip account
    let receiver = Pubkey::from_str("YOUR_RECIEVER_PUBKEY")?;
    let random_tip_account = jito_sdk.get_random_tip_account().await?;
    let jito_tip_account = Pubkey::from_str(&random_tip_account)?;

    // Define amounts to send (in lamports)
    let main_transfer_amount = 1_000; // 0.000001 SOL
    let jito_tip_amount = 1_000; // 0.000001 SOL
    let priority_fee_amount = 7_000; // 0.000007 SOL

    // Create instructions
    let prior_fee_ix =
        system_instruction::transfer(&sender.pubkey(), &jito_tip_account, priority_fee_amount);
    let main_transfer_ix =
        system_instruction::transfer(&sender.pubkey(), &receiver, main_transfer_amount);
    let jito_tip_ix =
        system_instruction::transfer(&sender.pubkey(), &jito_tip_account, jito_tip_amount);

    // Create transaction with all instructions
    let mut transaction = Transaction::new_with_payer(
        &[prior_fee_ix, main_transfer_ix, jito_tip_ix],
        Some(&sender.pubkey()),
    );

    // Get recent blockhash
    let recent_blockhash = solana_rpc.get_latest_blockhash()?;

    // Sign Transaction
    transaction.sign(&[&sender], recent_blockhash);

    // Serialize the full transaction
    let serialized_tx = general_purpose::STANDARD.encode(bincode::serialize(&transaction)?);

    // Send transaction using Jito SDK
    println!("Sending transaction...");
    let params = json!({
        "tx": serialized_tx
    });
    let response = jito_sdk.send_txn(Some(params), true).await?;

    // Extract signature from response
    let signature = response["result"]
        .as_str()
        .ok_or_else(|| anyhow!("Failed to get signature from response"))?;
    println!("Transaction sent with signature: {}", signature);

    // Confirm transaction
    let confirmation = solana_rpc.confirm_transaction_with_spinner(
        &signature.parse()?,
        &solana_rpc.get_latest_blockhash()?,
        CommitmentConfig::confirmed(),
    )?;
    println!("Transaction confirmed: {:?}", confirmation);

    println!(
        "View transaction on Solscan: https://solscan.io/tx/{}",
        signature
    );

    Ok(())
}
