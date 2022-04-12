mod webscraper;
use webscraper::find_urls::{WebScrapingError, index_urls, UrlIndex};


#[tokio::main]
async fn main() -> Result<(), WebScrapingError> {
    let result: UrlIndex = index_urls().await?;
    println!("URL Index: {:?}", result );
    Ok(())
}
