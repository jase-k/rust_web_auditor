mod webscraper;
use webscraper::find_urls::{WebScrapingError, visit_web};


#[tokio::main]
async fn main() -> Result<(), WebScrapingError> {
    Ok(visit_web().await?)
}
