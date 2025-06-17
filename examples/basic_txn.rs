use anyhow::{Result, anyhow};
use jito_sdk_rust::JitoJsonRpcSDK;
use solana_client::rpc_client::RpcClient;

use solana_pubkey::Pubkey;
use solana_keypair::Keypair;
use solana_signer::{Signer, EncodableKey};
use solana_program::system_instruction;
use solana_transaction::Transaction;
use solana_instruction::Instruction;

use base64::{Engine as _, engine::general_purpose};
use std::str::FromStr;
use serde_json::json;
use tracing::{info, debug};
use tracing_subscriber::EnvFilter;

fn init_tracing() {
    // This sets up logging with RUST_LOG environment variable
    // If RUST_LOG is not set, defaults to "info" level
    // Use RUST_LOG=off to disable logging entirely
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    init_tracing();

    // Set up Solana RPC client (for getting recent blockhash and confirming transaction)
    let solana_rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());

    // Setup client Jito Block Engine endpoint
    //let jito_sdk = JitoJsonRpcSDK::new("https://mainnet.block-engine.jito.wtf/api/v1", None);

    // Setup client Jito Block Engine endpoint with UUID
    //let uuid_string = "your-UUID-string".to_string();
    let uuid_string =  None;
    let jito_sdk = JitoJsonRpcSDK::new("https://mainnet.block-engine.jito.wtf/api/v1", uuid_string);
    
    // Load the sender's keypair - UPDATE THIS PATH to your actual wallet file
    // Common paths:
    // - Linux/Mac: "/home/username/.config/solana/id.json" 
    // - Or generate a test keypair: `solana-keygen new --outfile ./test-keypair.json`
    let wallet_path = std::env::var("/path/to/wallet-keypair.json")
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            format!("{}/.config/solana/id.json", home)
        });
    
    let sender = Keypair::read_from_file(&wallet_path)
        .map_err(|e| anyhow!(
            "Failed to read wallet file from '{}'. \n\
             Please either:\n\
             1. Generate a test keypair: solana-keygen new --outfile ./test-keypair.json\n\
             2. Set WALLET_PATH environment variable: export WALLET_PATH=./test-keypair.json\n\
             3. Use your existing Solana CLI keypair at ~/.config/solana/id.json\n\
             Error: {}", wallet_path, e
        ))?;
    
    info!("Sender pubkey: {}", sender.pubkey());

    // Set up receiver - UPDATE THIS to your actual recipient address
    let receiver = Pubkey::from_str(
        &std::env::var("RECIEVER_PUBKEY")
            .unwrap_or_else(|_| "11111111111111111111111111111112".to_string()) // System Program as default
    )?;
    
    // Get Jito tip account
    let random_tip_account = jito_sdk.get_random_tip_account().await?;
    let jito_tip_account = Pubkey::from_str(&random_tip_account)?;

    // Define amounts to send (in lamports)
    let main_transfer_amount = 1_000; // 0.000001 SOL
    let jito_tip_amount = 3_000; // 0.000003 SOL
    let priority_fee_amount: u64 = 700_000; // 0.000007 SOL in micro-lamports

    // SetComputeUnitPrice instruction: discriminator (3) + u64 value
    let compute_budget_program_id = Pubkey::from_str("ComputeBudget111111111111111111111111111111")?;
    let mut instruction_data = vec![3u8]; // SetComputeUnitPrice discriminator
    instruction_data.extend_from_slice(&priority_fee_amount.to_le_bytes());
    
    let set_compute_unit_price_ix = Instruction::new_with_bytes(
        compute_budget_program_id,
        &instruction_data,
        vec![],
    );

    // Create transfer instructions - system_instruction is in solana-program
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

    // Send transaction using Jito SDK (bundle_only = false for regular transaction)
    info!("Sending transaction...");
    let params = json!({
        "tx": serialized_tx
    });
    let response = jito_sdk.send_txn(Some(params), false).await?;

    // Extract signature from response
    let signature = response["result"]
        .as_str()
        .ok_or_else(|| anyhow!("Failed to get signature from response"))?;
    info!("Transaction sent with signature: {}", signature);

    // Confirm transaction using standard transaction confirmation (not bundle confirmation)
    debug!("Confirming transaction...");
    
    // Parse signature string to Signature type
    let signature_obj = signature.parse()
        .map_err(|e| anyhow!("Failed to parse signature: {}", e))?;
    
    // Standard transaction confirmation approach
    let max_retries = 30;
    let mut confirmed = false;
    
    for attempt in 1..=max_retries {
        match solana_rpc.get_signature_status(&signature_obj)? {
            Some(Ok(())) => {
                info!("Transaction confirmed successfully!");
                confirmed = true;
                break;
            },
            Some(Err(e)) => {
                return Err(anyhow!("Transaction failed: {:?}", e));
            },
            None => {
                debug!("Transaction not yet confirmed (attempt {}/{})", attempt, max_retries);
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
    }
    
    if !confirmed {
        return Err(anyhow!("Transaction not confirmed after {} attempts", max_retries));
    }

    info!("View transaction on Solscan: https://solscan.io/tx/{}", signature);

    Ok(())
}