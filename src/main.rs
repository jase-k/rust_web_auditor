mod webscraper;
use webscraper::find_urls::{index_urls, WebScrapingError};
use std::process::Command;

#[tokio::main]
async fn main() -> Result<(), WebScrapingError> {
    if cfg!(target_os = "windows") {
        println!("Running configuration for windows");
    } else if cfg!(target_os = "linux") {
        println!("Running configuration for linux");
    }

    index_urls(
        "https://f3d-shop.forgeflow.io/".to_string(),
        vec!["https://f3d-shop.forgeflow.io/".to_string()],
    )
    .await?;
    Ok(())
}
