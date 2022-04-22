mod webdriver;
mod webscraper;
use webscraper::find_urls::{index_urls, WebScrapingError};
use webdriver::webdriver::{DriverHandle, WebDriver};

#[tokio::main]
async fn main() -> Result<(), WebScrapingError> {
    if cfg!(target_os = "windows") {
        println!("Running configuration for windows");
    } else if cfg!(target_os = "linux") {
        println!("Running configuration for linux");
    }
    //Launches WebDriver
    let mut webdriver: DriverHandle = DriverHandle::new(WebDriver::GeckoDriver);

    index_urls(
        "https://example.com/".to_string(),
        vec!["https://example.com/".to_string()],
    )
    .await?;

    //Exits Gecko-Driver
    if let Err(e) =  webdriver.kill() {
        println!("Error closing Webdriver: {:?}", e);
    }

    Ok(())
}
