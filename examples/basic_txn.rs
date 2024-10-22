use anyhow::{Result, anyhow};
use jito_sdk_rust::JitoJsonRpcSDK;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
    compute_budget::ComputeBudgetInstruction,
};
use base64::{Engine as _, engine::general_purpose};
use std::str::FromStr;
use std::fs::File;
use std::io::BufReader;
use serde_json::json;

fn load_keypair(path: &str) -> Result<Keypair> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let wallet: Vec<u8> = serde_json::from_reader(reader)?;
    Ok(Keypair::from_bytes(&wallet)?)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set up Solana RPC client (for getting recent blockhash and confirming transaction)
    let solana_rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());

    // Setup client Jito Block Engine endpoint
    let jito_sdk = JitoJsonRpcSDK::new("https://mainnet.block-engine.jito.wtf/api/v1", None);

    // Setup client Jito Block Engine endpoint with UUID
    //let jito_sdk = JitoJsonRpcSDK::new("https://mainnet.block-engine.jito.wtf/api/v1", "UUID-API-KEY");
    
    // Load the sender's keypair
    let sender = load_keypair("/path/to/wallet.json" )?;
    
    println!("Sender pubkey: {}", sender.pubkey());

    // Set up receiver and Jito tip account
    let receiver = Pubkey::from_str("YOUR_RECIEVER_KEY")?;
    let random_tip_account = jito_sdk.get_random_tip_account().await?;
    let jito_tip_account = Pubkey::from_str(&random_tip_account)?;

    // Define amounts to send (in lamports)
    let main_transfer_amount = 1_000; // 0.000001 SOL
    let jito_tip_amount = 1_000; // 0.000001 SOL
    let priority_fee_amount = 7_000; // 0.000007 SOL


    // Create priority fee instruction
    let set_compute_unit_price_ix = ComputeBudgetInstruction::set_compute_unit_price(priority_fee_amount);

    // Create instructions
    let main_transfer_ix = system_instruction::transfer(
        &sender.pubkey(),
        &receiver,
        main_transfer_amount,
    );
    let jito_tip_ix = system_instruction::transfer(
        &sender.pubkey(),
        &jito_tip_account,
        jito_tip_amount,
    );

    // Create transaction with all instructions
    let mut transaction = Transaction::new_with_payer(
        &[set_compute_unit_price_ix, main_transfer_ix, jito_tip_ix],
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

    println!("View transaction on Solscan: https://solscan.io/tx/{}", signature);

    Ok(())
}