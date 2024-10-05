use anyhow::{anyhow, Result};
use jito_sdk_rust::JitoJsonRpcSDK;
use serde_json::json;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    signer::EncodableKey,
    system_instruction,
    transaction::Transaction,
};
use std::str::FromStr;
use tokio::time::{sleep, Duration};

#[derive(Debug)]
struct BundleStatus {
    confirmation_status: Option<String>,
    err: Option<serde_json::Value>,
    transactions: Option<Vec<String>>,
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

    // Create instructions
    let main_transfer_ix =
        system_instruction::transfer(&sender.pubkey(), &receiver, main_transfer_amount);
    let jito_tip_ix =
        system_instruction::transfer(&sender.pubkey(), &jito_tip_account, jito_tip_amount);

    // Create memo instruction
    let memo_program_id = Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")?;
    let memo_ix = Instruction::new_with_bytes(
        memo_program_id,
        b"hello world jito bundle",
        vec![AccountMeta::new(sender.pubkey(), true)],
    );

    // Create a transaction
    let mut transaction = Transaction::new_with_payer(
        &[main_transfer_ix, memo_ix, jito_tip_ix],
        Some(&sender.pubkey()),
    );

    // Get recent blockhash
    let recent_blockhash = solana_rpc.get_latest_blockhash()?;
    transaction.sign(&[&sender], recent_blockhash);

    // Serialize the transaction
    let serialized_tx = bs58::encode(bincode::serialize(&transaction)?).into_string();

    // Prepare bundle for submission (array of transactions)
    let bundle = json!([serialized_tx]);

    // UUID for the bundle
    let uuid = None;

    // Send bundle using Jito SDK
    println!("Sending bundle with 1 transaction...");
    let response = jito_sdk.send_bundle(Some(bundle), uuid).await?;

    // Extract bundle UUID from response
    let bundle_uuid = response["result"]
        .as_str()
        .ok_or_else(|| anyhow!("Failed to get bundle UUID from response"))?;
    println!("Bundle sent with UUID: {}", bundle_uuid);

    // Confirm bundle status
    let max_retries = 10;
    let retry_delay = Duration::from_secs(2);

    for attempt in 1..=max_retries {
        println!(
            "Checking bundle status (attempt {}/{})",
            attempt, max_retries
        );

        let status_response = jito_sdk
            .get_in_flight_bundle_statuses(vec![bundle_uuid.to_string()])
            .await?;

        if let Some(result) = status_response.get("result") {
            if let Some(value) = result.get("value") {
                if let Some(statuses) = value.as_array() {
                    if let Some(bundle_status) = statuses.get(0) {
                        if let Some(status) = bundle_status.get("status") {
                            match status.as_str() {
                                Some("Landed") => {
                                    println!("Bundle landed on-chain. Checking final status...");
                                    return check_final_bundle_status(&jito_sdk, bundle_uuid).await;
                                }
                                Some("Pending") => {
                                    println!("Bundle is pending. Waiting...");
                                }
                                Some(status) => {
                                    println!("Unexpected bundle status: {}. Waiting...", status);
                                }
                                None => {
                                    println!("Unable to parse bundle status. Waiting...");
                                }
                            }
                        } else {
                            println!("Status field not found in bundle status. Waiting...");
                        }
                    } else {
                        println!("Bundle status not found. Waiting...");
                    }
                } else {
                    println!("Unexpected value format. Waiting...");
                }
            } else {
                println!("Value field not found in result. Waiting...");
            }
        } else if let Some(error) = status_response.get("error") {
            println!("Error checking bundle status: {:?}", error);
        } else {
            println!("Unexpected response format. Waiting...");
        }

        if attempt < max_retries {
            sleep(retry_delay).await;
        }
    }

    Err(anyhow!(
        "Failed to confirm bundle status after {} attempts",
        max_retries
    ))
}

async fn check_final_bundle_status(jito_sdk: &JitoJsonRpcSDK, bundle_uuid: &str) -> Result<()> {
    let max_retries = 10;
    let retry_delay = Duration::from_secs(2);

    for attempt in 1..=max_retries {
        println!(
            "Checking final bundle status (attempt {}/{})",
            attempt, max_retries
        );

        let status_response = jito_sdk
            .get_bundle_statuses(vec![bundle_uuid.to_string()])
            .await?;
        let bundle_status = get_bundle_status(&status_response)?;

        match bundle_status.confirmation_status.as_deref() {
            Some("confirmed") => {
                println!("Bundle confirmed on-chain. Waiting for finalization...");
                check_transaction_error(&bundle_status)?;
            }
            Some("finalized") => {
                println!("Bundle finalized on-chain successfully!");
                check_transaction_error(&bundle_status)?;
                print_transaction_url(&bundle_status);
                return Ok(());
            }
            Some(status) => {
                println!(
                    "Unexpected final bundle status: {}. Continuing to poll...",
                    status
                );
            }
            None => {
                println!("Unable to parse final bundle status. Continuing to poll...");
            }
        }

        if attempt < max_retries {
            sleep(retry_delay).await;
        }
    }

    Err(anyhow!(
        "Failed to get finalized status after {} attempts",
        max_retries
    ))
}

fn get_bundle_status(status_response: &serde_json::Value) -> Result<BundleStatus> {
    status_response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(|value| value.as_array())
        .and_then(|statuses| statuses.get(0))
        .ok_or_else(|| anyhow!("Failed to parse bundle status"))
        .map(|bundle_status| BundleStatus {
            confirmation_status: bundle_status
                .get("confirmation_status")
                .and_then(|s| s.as_str())
                .map(String::from),
            err: bundle_status.get("err").cloned(),
            transactions: bundle_status
                .get("transactions")
                .and_then(|t| t.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                }),
        })
}

fn check_transaction_error(bundle_status: &BundleStatus) -> Result<()> {
    if let Some(err) = &bundle_status.err {
        if err["Ok"].is_null() {
            println!("Transaction executed without errors.");
            Ok(())
        } else {
            println!("Transaction encountered an error: {:?}", err);
            Err(anyhow!("Transaction encountered an error"))
        }
    } else {
        Ok(())
    }
}

fn print_transaction_url(bundle_status: &BundleStatus) {
    if let Some(transactions) = &bundle_status.transactions {
        if let Some(tx_id) = transactions.first() {
            println!("Transaction URL: https://solscan.io/tx/{}", tx_id);
        } else {
            println!("Unable to extract transaction ID.");
        }
    } else {
        println!("No transactions found in the bundle status.");
    }
}
