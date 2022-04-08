use fantoccini::elements::Element;
use fantoccini::error::{CmdError, NewSessionError};
use fantoccini::{Client, ClientBuilder, Locator};

#[derive(Debug)]
enum WebScrapingError {
    FantocciniNewSessionError(NewSessionError),
    FantocciniCmdErrorr(CmdError)
}

impl From<CmdError> for WebScrapingError {
    fn from (e: CmdError) -> Self {
        Self::FantocciniCmdErrorr(e)
    }
}

impl From<NewSessionError> for WebScrapingError {
    fn from (e: NewSessionError) -> Self {
        Self::FantocciniNewSessionError(e)
    }
}

#[tokio::main]
async fn main() -> Result<(), WebScrapingError> {
    Ok(visit_web().await?)
}

async fn open_new_client() -> Result<Client, WebScrapingError>  {
    Ok(ClientBuilder::native().connect("http://localhost:4444").await?)
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
            return Ok(all_urls)
        }
    }

}

async fn get_href(mut element: Element) -> Result<Option<String>, WebScrapingError> {
    Ok(element.attr("href").await?)
}

// async fn open_new_tab(web_client: &mut Client) -> Result<NewWindowResponse, WebScrapingError> {
//     web_client.new_window(true)
// }


#[cfg(test)]
mod tests {
    use super::*;
    use fantoccini::{Client};
    use futures::executor;

    struct MainClient {
        client: Option<Client>,
        connections: u8
    }

    enum TestErrors {
        TooManyConnections,
        ConnectionError
    }

    static MAIN_CLIENT: MainClient = MainClient { client: None, connections: 0 };
    
    impl MainClient {
        async fn connect(mainclient: &mut Self) -> Result<&Client, TestErrors> {
            if let Some(client_in_use) = &mainclient.client {
                Ok(mainclient.client.as_ref().unwrap())
            } else {
                let new_connection_result = ClientBuilder::native().connect("http://localhost:4444").await;
                
                match new_connection_result {
                    Ok(client) => {
                        mainclient.client = Some(client);
                        Ok(mainclient.client.as_ref().unwrap())
                    },
                    Err(e) => return Err(TestErrors::ConnectionError)
                }
            }
        }
    }

    // #[async_trait]
    impl Drop for MainClient {
        fn drop(&mut self){
            println!("Dropping Client");
            if let Some(client) = &self.client {
                executer::block_on(client.close());
            } else {
                println!("Error closing Client!");
            }
        }
    }


    //Open a client and edit drop function so that when client goes out of scope, close the connection. 


    // #[tokio::test]
    // async fn open_new_tab_test(){
    //     let new_tab_result = open_new_tab(TEST_CLIENT);
    // }

    #[tokio::test]
    async fn open_new_client_test(){
        
        let open_result = open_new_client().await;
        assert!(open_result.is_ok(), "Client did not OPEN properly {:?}", open_result);
        
        if let Ok(mut client) = open_result {
            let close_result = client.close().await;
            assert!(close_result.is_ok(), "Client did not CLOSE properly {:?}", close_result);
        }
    }

    #[tokio::test]
    async fn find_urls_test(){
        // Set-Up: 
        let open_result = open_new_client().await;
        if let Ok(mut client) = open_result {
            let url = "src/tests/supplement/four_urls.html";
            let nav_result = client.goto(&url).await;

            if let Err(res) = nav_result {
                let close_result = client.close().await;
                assert!(close_result.is_ok(), "Client did not CLOSE properly {:?}", close_result);
                assert!(false, "Error executing navigation to {}", url )
            }

            let close_result = client.close().await;
            assert!(close_result.is_ok(), "Client did not CLOSE properly {:?}", close_result);

        }
    }
    #[tokio::test]
    async fn find_urls_test1(){
        // Set-Up: 
        let open_result = open_new_client().await;
        if let Ok(mut client) = open_result {
            let url = "src/tests/supplement/four_urls.html";
            let nav_result = client.goto(&url).await;

            if let Err(res) = nav_result {
                let close_result = client.close().await;
                assert!(close_result.is_ok(), "Client did not CLOSE properly {:?}", close_result);
                assert!(false, "Error executing navigation to {}", url )
            }

            let close_result = client.close().await;
            assert!(close_result.is_ok(), "Client did not CLOSE properly {:?}", close_result);

        }
    }

    #[tokio::test]
    async fn find_urls_test2(){
        // Set-Up: 
        let open_result = open_new_client().await;
        if let Ok(mut client) = open_result {
            let url = "src/tests/supplement/four_urls.html";
            let nav_result = client.goto(&url).await;

            if let Err(res) = nav_result {
                let close_result = client.close().await;
                assert!(close_result.is_ok(), "Client did not CLOSE properly {:?}", close_result);
                assert!(false, "Error executing navigation to {}", url )
            }

            let close_result = client.close().await;
            assert!(close_result.is_ok(), "Client did not CLOSE properly {:?}", close_result);

        }
    }
}