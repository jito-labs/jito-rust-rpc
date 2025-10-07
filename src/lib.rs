
#[cfg(feature = "use-solana-types")]
pub mod solana_types;

use anyhow::{anyhow, Result};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::fmt;
use std::fmt::Display;
use std::sync::Arc;
use tracing::{debug, trace};
use rand::prelude::IndexedRandom;
use crate::JitoRpcErrorObject::RpcError;

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
        write!(f, "{}", serde_json::to_string_pretty(&self.0).unwrap())
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
    RpcError{ code: i64, message: String, http_status: StatusCode },
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
            JitoRpcErrorObject::RpcError { code, message, http_status } =>  write!(f, "RPC Error {}: {} (http status {})", code, message, http_status),
        }
    }
}

impl std::error::Error for JitoRpcErrorObject {}


impl JitoJsonRpcSDK {
    pub fn new(base_url: &str, jito_auth_uuid: Option<String>) -> Self {
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
            "params": params.unwrap_or(json!([]))
        });

        trace!("Sending request to: {}", url);
        trace!(
            "Request body: {}",
            serde_json::to_string_pretty(&data).unwrap()
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&data)
            .send()
            .await?;

        let status = response.status();
        debug!("Response status: {}", status);

        let body = response.json::<Value>().await?;
        if tracing::level_enabled!(tracing::Level::TRACE) {
            trace!(
                "Raw response body: {}",
                serde_json::to_string_pretty(&body).unwrap()
            );
        }

        // handle error independently of http status code
        if body["error"].is_object() {
            let error_object = body["error"].as_object().unwrap();
            if let (Some(code), Some(message)) = (error_object.get("code"), error_object.get("message")) {
                let code: Option<i64> = code.as_i64();
                let message: Option<&str> = message.as_str();
                let http_status = status;
                // note: we assume that
                trace!("Error code: {:?}, message: {:?}, http status: {}", code, message, http_status);

                return Err(RpcError {
                    code: code.unwrap_or_default(),
                    message: message.unwrap_or_default().to_string(),
                    http_status,
                })
            }

        }

        Ok(body)
    }

    pub async fn get_tip_accounts(&self) -> Result<Value, JitoRpcErrorObject>{
        let endpoint = if let Some(uuid) = &self.jito_auth_uuid {
            format!("/bundles?uuid={}", uuid)
        } else {
            "/bundles".to_string()
        };

        self.send_request(&endpoint, "getTipAccounts", None).await
    }

    // Get a random tip account
    pub async fn get_random_tip_account(&self) -> Result<String> {
        let tip_accounts_response = self.get_tip_accounts().await?;

        let tip_accounts = tip_accounts_response["result"]
            .as_array()
            .ok_or_else(|| anyhow!("Failed to parse tip accounts as array"))?;

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
            format!("/getBundleStatuses?uuid={}", uuid)
        } else {
            "/getBundleStatuses".to_string()
        };

        // Construct the params as a list within a list
        let params = json!([bundle_uuids]);

        self.send_request(&endpoint, "getBundleStatuses", Some(params))
            .await
            .map_err(|e| anyhow!("Request error: {}", e))
    }

    pub async fn send_bundle_base64(&self, txlist_encoded: Vec<String>) -> Result<Value, anyhow::Error> {
        let mut endpoint = "/bundles".to_string();

        if let Some(uuid) = self.jito_auth_uuid.as_deref() {
            endpoint = format!("{}?uuid={}", endpoint, uuid);
        }

        let request_params = json!([
            txlist_encoded,
            {
                "encoding": "base64"
            }
        ]);

        self.send_request(&endpoint, "sendBundle", Some(request_params))
            .await
            .map_err(|e| anyhow!("Request error: {}", e))
    }

    pub async fn send_bundle(&self, params: Option<Value>, jito_auth_uuid: Option<&str>) -> Result<Value, anyhow::Error> {
        let mut endpoint = "/bundles".to_string();
        
        if let Some(uuid) = jito_auth_uuid {
            endpoint = format!("{}?uuid={}", endpoint, uuid);
        }
    
        // Create the parameters for the request
        let request_params = match params {
            // If params is already in the correct format [transactions, {encoding: "base64"}]
            Some(ref value) if value.is_array() && value.as_array().unwrap().len() == 2 => {
                // Use it as is
                value.clone()
            },
            Some(Value::Array(transactions)) => {
                // Validate transactions
                if transactions.is_empty() {
                    return Err(anyhow!("Bundle must contain at least one transaction"));
                }
                if transactions.len() > 5 {
                    return Err(anyhow!("Bundle can contain at most 5 transactions"));
                }
                
                json!([
                    transactions,
                    {
                        "encoding": "base64"
                    }
                ])
            },
            _ => return Err(anyhow!("Invalid bundle format: expected an array of transactions")),
        };
    
        self.send_request(&endpoint, "sendBundle", Some(request_params))
            .await
            .map_err(|e| anyhow!("Request error: {}", e))
    }

    pub async fn send_txn(&self, params: Option<Value>, bundle_only: bool) -> Result<Value, JitoRpcErrorObject> {
        let mut query_params = Vec::new();

        if bundle_only {
            query_params.push("bundleOnly=true".to_string());
        }

        let endpoint = if query_params.is_empty() {
            "/transactions".to_string()
        } else {
            format!("/transactions?{}", query_params.join("&"))
        };

        let params = match params {
            Some(Value::Object(map)) => {
                let tx = map.get("tx").and_then(Value::as_str).unwrap_or_default();
                let skip_preflight = map.get("skipPreflight").and_then(Value::as_bool).unwrap_or(false);
                json!([
                    tx,
                    {
                        "encoding": "base64",
                        "skipPreflight": skip_preflight
                    }
                ])
            },
            _ => json!([]),
        };

        self.send_request(&endpoint, "sendTransaction", Some(params)).await
    }

    pub async fn get_in_flight_bundle_statuses(&self, bundle_uuids: Vec<String>) -> Result<Value> {
        let endpoint = if let Some(uuid) = &self.jito_auth_uuid {
            format!("/getInflightBundleStatuses?uuid={}", uuid)
        } else {
            "/getInflightBundleStatuses".to_string()
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

