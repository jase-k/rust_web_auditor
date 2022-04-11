use fantoccini::elements::Element;
use fantoccini::error::{CmdError, NewSessionError};
use fantoccini::{Client, ClientBuilder, Locator};

#[derive(Debug)]
enum WebScrapingError {
    FantocciniNewSessionError(NewSessionError),
    FantocciniCmdErrorr(CmdError),
}

impl From<CmdError> for WebScrapingError {
    fn from(e: CmdError) -> Self {
        Self::FantocciniCmdErrorr(e)
    }
}

impl From<NewSessionError> for WebScrapingError {
    fn from(e: NewSessionError) -> Self {
        Self::FantocciniNewSessionError(e)
    }
}

#[tokio::main]
async fn main() -> Result<(), WebScrapingError> {
    Ok(visit_web().await?)
}

async fn open_new_client() -> Result<Client, WebScrapingError> {
    Ok(ClientBuilder::native()
        .connect("http://localhost:4444")
        .await?)
}

async fn visit_web() -> Result<(), WebScrapingError> {
    let mut web_client = open_new_client().await?;

    web_client.goto("https://lulzbot.com/").await?; // ? since this function returns an Result Type. This ? unwraps result, but if returns an err, it'll stop the whole function and return the error.
    let url = web_client.current_url().await?;
    println!("{}", url.as_ref());

    let _all_urls = find_urls(&mut web_client).await?;

    Ok(web_client.close().await?)
}

async fn find_urls(web_client: &mut Client) -> Result<Vec<String>, WebScrapingError> {
    let locator = Locator::XPath("//a");

    let all_anchors = web_client.find_all(locator).await?; //Vec<Elements>
    let mut all_anchors_iter = all_anchors.iter();

    let mut all_urls: Vec<String> = Vec::new();

    loop {
        if let Some(element) = all_anchors_iter.next() {
            if let Some(url) = get_href(element.clone()).await? {
                all_urls.push(url);
            };
        } else {
            return Ok(all_urls);
        }
    }
}

async fn get_href(mut element: Element) -> Result<Option<String>, WebScrapingError> {
    Ok(element.attr("href").await?)
}

// async fn open_new_tab(web_client: &mut Client) -> Result<NewWindowResponse, WebScrapingError> {
//     web_client.new_window(true)
// }