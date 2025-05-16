use jito_sdk_rust::JitoJsonRpcSDK;
use tracing::{info, error};
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    init_tracing();

    // Example with no UUID (default)
    let sdk = JitoJsonRpcSDK::new("https://mainnet.block-engine.jito.wtf/api/v1", None);

    // Example with UUID (for rate limit approved)
    // let uuid_string = "your-uuid-here".to_string();
    // let sdk = JitoJsonRpcSDK::new("https://mainnet.block-engine.jito.wtf/api/v1", Some(uuid_string));
    
    match sdk.get_tip_accounts().await {
        Ok(tip_accounts) => {
            let pretty_tip_accounts = JitoJsonRpcSDK::prettify(tip_accounts);
            info!("Tip accounts:\n{}", pretty_tip_accounts);
        },
        Err(e) => error!("Error fetching tip accounts: {:?}", e),
    }
 
    // Example of getting a random tip account
    // let random_tip_account = sdk.get_random_tip_account().await?;
    // info!("Random tip account: {}", random_tip_account);
    
    Ok(())
}