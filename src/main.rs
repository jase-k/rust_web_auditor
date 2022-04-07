use fantoccini::{ClientBuilder, Locator};

#[tokio::main]
async fn main() {
    visit_web().await;
}

async fn visit_web() -> Result<(), fantoccini::error::CmdError> {
    let mut web_driver = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("failed to connect to WebDriver");

    web_driver.goto("https://lulzbot.com/").await?; // ? since this function returns an Result Type. This ? unwraps result, but if returns an err, it'll stop the whole function and return the error.
    let url = web_driver.current_url().await?;
    println!("{}", url.as_ref());

    web_driver.close().await
}
