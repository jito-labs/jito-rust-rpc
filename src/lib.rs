#[cfg(feature = "use-solana-types")]
pub mod solana_types;

use anyhow::{anyhow, Result};
use rand::prelude::IndexedRandom;
use reqwest::{Client, Response, StatusCode};
use serde_json::{json, Value};
use std::fmt;
use std::fmt::Display;
use std::sync::Arc;
use tracing::{debug, trace};

#[derive(Clone)]
pub struct JitoJsonRpcSDK {
    base_url: String,
    jito_auth_uuid: Option<String>,
    client: Client,
}

#[derive(Debug)]
pub struct PrettyJsonValue(pub Value);

impl fmt::Display for PrettyJsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_json::to_string_pretty(&self.0) {
            Ok(pretty) => write!(f, "{}", pretty),
            Err(_) => write!(f, "<invalid JSON>"),
        }
    }
}

impl From<Value> for PrettyJsonValue {
    fn from(value: Value) -> Self {
        PrettyJsonValue(value)
    }
}

#[derive(Clone, Debug)]
pub enum JitoRpcErrorObject {
    HttpError(Arc<reqwest::Error>),
    RpcError {
        code: i64,
        message: String,
        http_status: StatusCode,
    },
}

impl From<reqwest::Error> for JitoRpcErrorObject {
    fn from(err: reqwest::Error) -> Self {
        JitoRpcErrorObject::HttpError(Arc::new(err))
    }
}

impl Display for JitoRpcErrorObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JitoRpcErrorObject::HttpError(err) => write!(f, "HTTP Error: {}", err),
            JitoRpcErrorObject::RpcError {
                code,
                message,
                http_status,
            } => write!(
                f,
                "RPC Error {}: {} (http status {})",
                code, message, http_status
            ),
        }
    }
}

impl std::error::Error for JitoRpcErrorObject {}

impl JitoJsonRpcSDK {
    /// base_url example: "https://mainnet.block-engine.jito.wtf"
    pub fn new_with_base_url(base_url: &str, jito_auth_uuid: Option<String>) -> Self {
        assert!(
            !base_url.ends_with("/api/v1"),
            "Base URL must NOT include the version"
        );
        assert!(
            !base_url.ends_with("/"),
            "Base URL must NOT end with a slash"
        );
        Self {
            base_url: base_url.to_string(),
            jito_auth_uuid,
            client: Client::new(),
        }
    }

    async fn send_request(
        &self,
        endpoint: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, JitoRpcErrorObject> {
        let url = format!("{}{}", self.base_url, endpoint);

        let data = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params.unwrap_or_else(|| json!([]))
        });

        // Only log if the corresponding tracing level is enabled
        if tracing::enabled!(tracing::Level::TRACE) {
            trace!("Sending request to: {}", url);
            if let Ok(pretty) = serde_json::to_string_pretty(&data) {
                trace!("Request body:\n{}", pretty);
            }
        }

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&data)
            .send()
            .await?;

        if tracing::enabled!(tracing::Level::DEBUG) {
            debug!("Response status: {}", response.status());
        }

        check_response_and_return(response).await
    }

    pub async fn get_tip_accounts(&self) -> Result<Value, JitoRpcErrorObject> {
        let endpoint = if let Some(uuid) = &self.jito_auth_uuid {
            format!("/api/v1/bundles?uuid={}", uuid)
        } else {
            "/api/v1/bundles".to_string()
        };

        self.send_request(&endpoint, "getTipAccounts", None).await
    }

    // Get a random tip account
    pub async fn get_random_tip_account(&self) -> Result<String> {
        let tip_accounts_response = self.get_tip_accounts().await?;

        let tip_accounts = tip_accounts_response
            .get("result")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Failed to parse 'result' as an array of tip accounts"))?;

        if tip_accounts.is_empty() {
            return Err(anyhow!("No tip accounts available"));
        }

        let random_account = tip_accounts
            .choose(&mut rand::rng())
            .ok_or_else(|| anyhow!("Failed to choose random tip account"))?;

        random_account
            .as_str()
            .ok_or_else(|| anyhow!("Failed to parse tip account as string"))
            .map(String::from)
    }

    pub async fn get_bundle_statuses(&self, bundle_uuids: Vec<String>) -> Result<Value> {
        let endpoint = if let Some(uuid) = &self.jito_auth_uuid {
            format!("/api/v1/getBundleStatuses?uuid={}", uuid)
        } else {
            "/api/v1/getBundleStatuses".to_string()
        };

        // Construct the params as a list within a list
        let params = json!([bundle_uuids]);

        self.send_request(&endpoint, "getBundleStatuses", Some(params))
            .await
            .map_err(|e| anyhow!("Request error: {}", e))
    }

    pub async fn send_bundle_base64(
        &self,
        encoded_txs: Vec<String>,
    ) -> Result<Value, anyhow::Error> {
        if encoded_txs.is_empty() {
            return Err(anyhow!(
                "Transaction bundle is empty: expected at least one transaction to send"
            ));
        }
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

    pub async fn send_bundle(
        &self,
        params: Option<Value>,
        jito_auth_uuid: Option<&str>,
    ) -> Result<Value, anyhow::Error> {
        // Construct the endpoint
        let endpoint = match jito_auth_uuid {
            Some(uuid) => format!("/api/v1/bundles?uuid={uuid}"),
            None => "/api/v1/bundles".to_string(),
        };

        // Prepare request parameters
        let request_params = match params {
            Some(Value::Array(ref arr)) if arr.len() == 2 => {
                // Assume already in correct format: [transactions, {"encoding": "base64"}]
                Value::Array(arr.clone())
            }
            // Note : this is matched when we just send txs as an array like params : [tx1,tx2..]
            Some(Value::Array(transactions)) => {
                if transactions.is_empty() {
                    return Err(anyhow!("Bundle must contain at least one transaction"));
                }
                if transactions.len() > 5 {
                    return Err(anyhow!("Bundle can contain at most 5 transactions"));
                }

                json!([
                    transactions,
                    { "encoding": "base64" }
                ])
            }
            _ => {
                return Err(anyhow!(
                    "Invalid bundle format: expected an array of transactions"
                ))
            }
        };

        // Send the RPC request
        self.send_request(&endpoint, "sendBundle", Some(request_params))
            .await
            .map_err(|err| anyhow!("Failed to send bundle: {err:#}"))
    }

    pub async fn send_txn(
        &self,
        params: Option<Value>,
        bundle_only: bool,
    ) -> Result<Value, JitoRpcErrorObject> {
        let endpoint = {
            let query_param = if bundle_only {
                Some("bundleOnly=true")
            } else {
                None
            };
            match query_param {
                Some(q) => format!("/api/v1/transactions?{}", q),
                None => "/api/v1/transactions".to_string(),
            }
        };

        let params = match params {
            Some(Value::Object(map)) => {
                let tx = map.get("tx").and_then(Value::as_str).unwrap_or_default();
                let skip_preflight = map
                    .get("skipPreflight")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                json!([
                    tx,
                    {
                        "encoding": "base64",
                        "skipPreflight": skip_preflight
                    }
                ])
            }
            _ => json!([]),
        };

        self.send_request(&endpoint, "sendTransaction", Some(params))
            .await
    }

    pub async fn get_in_flight_bundle_statuses(&self, bundle_uuids: Vec<String>) -> Result<Value> {
        let endpoint = if let Some(uuid) = &self.jito_auth_uuid {
            format!("/api/v1/getInflightBundleStatuses?uuid={}", uuid)
        } else {
            "/api/v1/getInflightBundleStatuses".to_string()
        };

        let params = json!([bundle_uuids]);

        self.send_request(&endpoint, "getInflightBundleStatuses", Some(params))
            .await
            .map_err(|e| anyhow!("Request error: {}", e))
    }

    // Helper method
    pub fn prettify(value: Value) -> PrettyJsonValue {
        PrettyJsonValue(value)
    }
}

async fn check_response_and_return(response: Response) -> Result<Value, JitoRpcErrorObject> {
    let status = response.status();

    if tracing::level_enabled!(tracing::Level::DEBUG) {
        debug!("Response status: {}", status);
    }

    let body: Value = response.json().await?;

    if tracing::level_enabled!(tracing::Level::TRACE) {
        if let Ok(pretty) = serde_json::to_string_pretty(&body) {
            trace!("Raw response body:\n{pretty}");
        }
    }

    if let Some(error_obj) = body.get("error").and_then(|e| e.as_object()) {
        let code = error_obj
            .get("code")
            .and_then(|v| v.as_i64())
            .unwrap_or_default();

        let message = error_obj
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string();

        // note: we assume that
        if tracing::level_enabled!(tracing::Level::TRACE) {
            trace!(
                "Jito RPC returned error: code = {}, message = \"{}\", http_status = {}",
                code,
                message,
                status
            );
        }

        return Err(JitoRpcErrorObject::RpcError {
            code,
            message,
            http_status: status,
        });
    }

    Ok(body)
}
