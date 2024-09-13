use jito_sdk_rust::JitoJsonRpcSDK;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //Example with no UUID(default)
    let sdk = JitoJsonRpcSDK::new("https://mainnet.block-engine.jito.wtf/api/v1", None);

    //Example with UUID(for rate limit approved)
    //let sdk = JitoJsonRpcSDK::new("https://mainnet.block-engine.jito.wtf/api/v1", <YOUR_UUID_VALUE>);
    
    match sdk.get_tip_accounts().await {
        Ok(tip_accounts) => {
            let pretty_tip_accounts = JitoJsonRpcSDK::prettify(tip_accounts);
            println!("Tip accounts:\n{}", pretty_tip_accounts);
        },
        Err(e) => eprintln!("Error: {:?}", e),
    }
 
    /*
    let random_tip_account = sdk.get_random_tip_account().await?;
    println!("Random tip account: {}", random_tip_account);
    */
    Ok(())
}