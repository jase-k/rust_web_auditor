mod webscraper;
use webscraper::find_urls::{index_urls, UrlIndex, WebScrapingError};

#[tokio::main]
async fn main() -> Result<(), WebScrapingError> {
    let result: UrlIndex = index_urls().await?;
    println!("URL Index: {:?}", result);
    Ok(())
}
