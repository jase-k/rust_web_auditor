mod webscraper;
use webscraper::find_urls::{index_urls, WebScrapingError};
use std::fs;
 use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), WebScrapingError> {
    let result = index_urls(
        "https://f3d-shop.forgeflow.io/".to_string(),
        vec!["https://f3d-shop.forgeflow.io/".to_string()],
    )
    .await;
    match result {
        Ok(res) => {
            println!("{:?}", res);
            // Print to Files
            // let res_iter = res.into_values();
            if let Ok(mut good_urls_file) = fs::File::options().append(false).create(true).open("./good_urls.json") {
                if let Ok(string) = serde_json::to_string(&res) {
                    good_urls_file.write(string.as_bytes());
                } 
            }
            Ok(())
        },
        Err(e) => Err(e) 
    } 
}
