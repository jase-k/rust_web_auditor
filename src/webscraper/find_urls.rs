use fantoccini::elements::Element;
use fantoccini::error::{CmdError, NewSessionError};
use fantoccini::{Client, ClientBuilder, Locator};
// use no_deadlocks::{Mutex}; // Switch out to std::sync before build
use std::sync::{Arc, Mutex};


#[derive(Debug)]
pub enum WebScrapingError {
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

#[derive(Debug, Clone)]
pub struct Url {
    url: String, 
    response_code: u16, 
    site_references: Arc<Mutex<Vec<String>>>, 
    redirected_to: Box<Option<Url>>
}

impl Url {
    fn new(url: String, response_code: u16, site_reference: String) -> Url {
        let mut new_vec = Vec::new();
        new_vec.push(site_reference);
        Url {
            url: url, 
            response_code: response_code,
            site_references: Arc::new(Mutex::new(new_vec)),
            redirected_to: Box::new(None)
        }
    }
    
    fn add_reference(&self, site_reference: String) -> &Self {
        let lock_result = self.site_references.lock();
        
        if let Ok(mut mutex_guard) = lock_result {
            (*mutex_guard).push(site_reference);
        }
        println!("{:?}", self);
        self
    }
    //TODO: add redirection
}

impl PartialEq<Url> for Url {
    fn eq(&self, other: &Url) -> bool {
        if self.url != other.url {
            return false
        }
        if self.response_code != other.response_code {
            return false
        }

        let self_vec = self.site_references.lock().unwrap();
        let other_vec = other.site_references.lock().unwrap();
        *self_vec == *other_vec
    }
}

#[derive(Debug)]
pub struct UrlIndex {
    bad_urls: Arc<Mutex<Vec<Url>>>, //400-499 response status
    good_urls: Arc<Mutex<Vec<Url>>>,  //200-299 response status 
    redirected_urls: Arc<Mutex<Vec<Url>>>, //300-399 response status
    error_urls: Arc<Mutex<Vec<Url>>> //500+ response status Internal errors. 
}

impl UrlIndex {
    fn new() -> UrlIndex {
        UrlIndex {
            bad_urls: Arc::new(Mutex::new(vec![])), 
            good_urls: Arc::new(Mutex::new(vec![])),  
            redirected_urls: Arc::new(Mutex::new(vec![])), 
            error_urls: Arc::new(Mutex::new(vec![]))
        }
    }

    fn add(&self, url: Url) -> &Self {
        let mut url_vector = self.good_urls.lock().unwrap();
        (*url_vector).push(url);
        self
    }
}

impl PartialEq for UrlIndex {
    fn eq(&self, other: &UrlIndex) -> bool {

        let self_bad_urls = self.bad_urls.lock().unwrap();
        let other_bad_urls = other.bad_urls.lock().unwrap();
        if *self_bad_urls != *other_bad_urls {
            return false
        }

        let self_good_urls = self.good_urls.lock().unwrap();
        let other_good_urls = other.good_urls.lock().unwrap();
        if *self_good_urls != *other_good_urls {
            return false
        }

        let self_redirected_urls = self.redirected_urls.lock().unwrap();
        let other_redirected_urls = other.redirected_urls.lock().unwrap();
        if *self_redirected_urls != *other_redirected_urls {
            return false
        }

        let self_error_urls = self.error_urls.lock().unwrap();
        let other_error_urls = other.error_urls.lock().unwrap();
        if *self_error_urls != *other_error_urls {
            return false
        }

        return true
    }
}


pub async fn index_urls() -> Result<UrlIndex, WebScrapingError> {
    // Open web connection
    // Create up to 10 new pages
    // For each url -> parse urls and add to set. 
        // -> parse : (response status, site reference, and url) 
    // Err(WebScrapingError::FantocciniCmdErrorr(CmdError))
    Ok(UrlIndex{
        bad_urls: Arc::new(Mutex::new(Vec::new())) , //400-499 response status
        good_urls: Arc::new(Mutex::new(Vec::new())) ,  //200-299 response status 
        redirected_urls: Arc::new(Mutex::new(Vec::new())) , //300-399 response status
        error_urls: Arc::new(Mutex::new(Vec::new())) 
    })
}

pub async fn visit_web() -> Result<(), WebScrapingError> {
    let mut web_client = open_new_client().await?;

    web_client.goto("https://lulzbot.com/").await?; // ? since this function returns an Result Type. This ? unwraps result, but if returns an err, it'll stop the whole function and return the error.
    let url = web_client.current_url().await?;
    println!("{}", url.as_ref());

    let _all_urls = find_urls(&mut web_client).await?;

    Ok(web_client.close().await?)
}

async fn open_new_client() -> Result<Client, WebScrapingError> {
    Ok(ClientBuilder::native()
        .connect("http://localhost:4444")
        .await?)
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

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn url_new_test(){
        let url = Url::new("https://example.com".to_string(), 200, "https://google.com/".to_string());
        assert_eq!(url, Url{
            url: "https://example.com".to_string(), 
            response_code: 200, 
            site_references: Arc::new(Mutex::new(vec!["https://google.com/".to_string()])), 
            redirected_to: Box::new(None)
        })
    }

    #[test]
    fn url_add_reference_test(){
        let url = Url::new("https://example.com".to_string(), 200, "https://google.com/".to_string());
        url.add_reference("https://facebook.com/".to_string());
        assert_eq!(url, Url{
            url: "https://example.com".to_string(), 
            response_code: 200, 
            site_references: Arc::new(Mutex::new(vec!["https://google.com/".to_string(), "https://facebook.com/".to_string()])),
            redirected_to: Box::new(None)
        })
    }

    #[test]
    fn url_index_new_test(){
        let url_index = UrlIndex::new();
        assert_eq!(url_index, UrlIndex {
            bad_urls: Arc::new(Mutex::new(Vec::new())), 
            good_urls: Arc::new(Mutex::new(Vec::new())),  
            redirected_urls: Arc::new(Mutex::new(Vec::new())), 
            error_urls: Arc::new(Mutex::new(Vec::new()))
        })
    }

    #[test]
    fn url_index_add_good_url_test(){
        let url_index = UrlIndex::new();
        let url = Url::new("https://example.com".to_string(), 200, "https://google.com/".to_string());
        let url_copy = url.clone();

        url_index.add(url); 

        assert_eq!(url_index, UrlIndex {
            bad_urls: Arc::new(Mutex::new(Vec::new())), 
            good_urls: Arc::new(Mutex::new(vec![url_copy])),  
            redirected_urls: Arc::new(Mutex::new(Vec::new())), 
            error_urls: Arc::new(Mutex::new(Vec::new()))
        })
    }
}