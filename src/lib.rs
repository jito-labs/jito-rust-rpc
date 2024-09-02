use reqwest::Client;
use serde_json::{json, Value};
use std::fmt;

pub struct JitoJsonRpcSDK {
    base_url: String,
    uuid: Option<String>,
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

impl JitoJsonRpcSDK {
    pub fn new(base_url: &str, uuid: Option<String>) -> Self {
        Self {
            base_url: base_url.to_string(),
            uuid,
            client: Client::new(),
        }
    }

    async fn send_request(&self, endpoint: &str, method: &str, params: Option<Value>) -> Result<Value, reqwest::Error> {
        let url = format!("{}{}", self.base_url, endpoint);
        
        let data = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params.unwrap_or(json!([]))
        });

        println!("Sending request to: {}", url);
        println!("Request body: {}", serde_json::to_string_pretty(&data).unwrap());

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&data)
            .send()
            .await?;

        let status = response.status();
        println!("Response status: {}", status);

        let body = response.json::<Value>().await?;
        println!("Response body: {}", serde_json::to_string_pretty(&body).unwrap());

        Ok(body)
    }

    pub async fn get_tip_accounts(&self) -> Result<Value, reqwest::Error> {
        let endpoint = if let Some(uuid) = &self.uuid {
            format!("/bundles?uuid={}", uuid)
        } else {
            "/bundles".to_string()
        };

        self.send_request(&endpoint, "getTipAccounts", None).await
    }

    pub async fn get_bundle_statuses(&self, params: Option<Value>) -> Result<Value, reqwest::Error> {
        let endpoint = if let Some(uuid) = &self.uuid {
            format!("/bundles?uuid={}", uuid)
        } else {
            "/bundles".to_string()
        };

        self.send_request(&endpoint, "getBundleStatuses", params).await
    }

    pub async fn send_bundle(&self, params: Option<Value>) -> Result<Value, reqwest::Error> {
        let endpoint = if let Some(uuid) = &self.uuid {
            format!("/bundles?uuid={}", uuid)
        } else {
            "/bundles".to_string()
        };

        self.send_request(&endpoint, "sendBundle", params).await
    }

    pub async fn send_txn(&self, params: Option<Value>, bundle_only: bool) -> Result<Value, reqwest::Error> {
        let mut query_params = Vec::new();

        if bundle_only {
            query_params.push("bundleOnly=true".to_string());
        }

        if let Some(uuid) = &self.uuid {
            query_params.push(format!("uuid={}", uuid));
        }

        let endpoint = if query_params.is_empty() {
            "/transactions".to_string()
        } else {
            format!("/transactions?{}", query_params.join("&"))
        };

        self.send_request(&endpoint, "sendTransaction", params).await
    }

    // Helper method to convert Value to PrettyJsonValue
    pub fn prettify(value: Value) -> PrettyJsonValue {
        PrettyJsonValue(value)
    }
}