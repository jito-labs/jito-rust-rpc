use crate::JitoJsonRpcSDK;
use anyhow::{anyhow, bail};
use base64::engine::general_purpose;
use base64::Engine;
use serde_json::{json, Value};
use solana_transaction::versioned::VersionedTransaction;

impl JitoJsonRpcSDK {
    pub async fn send_bundle_of_transactions(
        &self,
        transactions: &[VersionedTransaction],
    ) -> anyhow::Result<Value, anyhow::Error> {
        let encoded_txs = convert_transactions_to_base64(transactions)?;

        let mut endpoint = "/api/v1/bundles".to_string();

        if let Some(uuid) = self.jito_auth_uuid.as_deref() {
            endpoint = format!("{}?uuid={}", endpoint, uuid);
        }

        let request_params = json!([
            encoded_txs,
            {
                "encoding": "base64"
            }
        ]);

        self.send_request(&endpoint, "sendBundle", Some(request_params))
            .await
            .map_err(|e| anyhow!("Request error: {}", e))
    }
}

fn convert_transactions_to_base64(
    transactions: &[VersionedTransaction],
) -> anyhow::Result<Vec<String>, anyhow::Error> {
    let mapped: Vec<Option<Vec<u8>>> = transactions
        .iter()
        .map(|tx| (bincode::serialize(tx).ok()))
        .collect();

    let failed_idx: Vec<usize> = mapped
        .iter()
        .enumerate()
        .filter(|(_, opt)| opt.is_none())
        .map(|(i, _)| i)
        .collect();

    if !failed_idx.is_empty() {
        bail!(
            "Failed to serialize transactions at indices: {:?}",
            failed_idx
        );
    }

    let base64: Vec<String> = mapped
        .iter()
        .flatten()
        .map(|b| general_purpose::STANDARD.encode(b))
        .collect();

    Ok(base64)
}
