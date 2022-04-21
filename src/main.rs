mod webscraper;
use webscraper::find_urls::{index_urls, WebScrapingError};

#[tokio::main]
async fn main() -> Result<(), WebScrapingError> {
    index_urls("https://f3d-shop.forgeflow.io/".to_string(), vec!["https://f3d-shop.forgeflow.io/".to_string()]).await?;
    Ok(())
}
