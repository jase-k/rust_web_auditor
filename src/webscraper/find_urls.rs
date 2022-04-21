use fantoccini::elements::Element;
use fantoccini::error::{CmdError, NewSessionError};
use fantoccini::{Client, ClientBuilder, Locator};
use std::collections::{ HashSet, HashMap };

#[derive(Debug)]
pub enum WebScrapingError {
    FantocciniNewSessionError(NewSessionError),
    FantocciniCmdErrorr(CmdError),
    FindingDomainError,
    FormattingUrlError
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

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Url {
    response_code: Option<u16>,
    full_path: String,
    site_references: Vec<String>,
    redirected_to: Option<String>,
}

impl Url {
    fn new(url: String, response_code: Option<u16>, site_reference: String) -> Url {
        let mut new_vec = Vec::new();
        new_vec.push(site_reference);
        Url {
            full_path: url,
            response_code: response_code,
            site_references: new_vec,
            redirected_to: None,
        }
    }

    fn add_reference(&mut self, site_reference: String) -> &Self {
        self.site_references.push(site_reference);
        self
    }

    fn set_redirection(&mut self, destination: String) -> &Self {
        self.redirected_to = Some(destination);
        self
    }

    async fn set_response_code(&mut self, web_client: &mut Client) -> Result<(), WebScrapingError> {
        let current_url = web_client.current_url().await?;

        if is_404(web_client).await? {
            self.response_code = Some(404);
            println!("Response 404 from: {}", self.full_path);
        } else if self.full_path == current_url.as_str() {
            self.response_code = Some(200);
            println!("Response 200 from: {}", self.full_path);
        } else {
            self.response_code = Some(300);
            self.set_redirection(current_url.to_string());
            println!("Response 300 from: {}", self.full_path);
        };
        Ok(())
    }
}






/// # Purpose
/// Return an result with Ok(UrlIndex)
pub async fn index_urls(starting_url: String, domains: Vec<String>) -> Result<(), WebScrapingError> {
    let first_url = Url::new(starting_url.clone(), None, starting_url.clone());

    let mut url_hash_set: HashSet<String> = HashSet::from([starting_url.clone()]);
    
    let mut url_index: HashMap<String, Url> = HashMap::from([
            (starting_url, first_url),
            ]);

    // Open web connection with webdriver (fantoccinni crate)
    println!("Opening Up Web Client");
    let mut web_client: Client = open_new_client().await?;
    println!("Connected to Web Client");
    
    let final_index: HashMap<String, Url> = create_index(url_index, url_hash_set, domains, &mut web_client).await?;
    
    // TODO: 
    //Print url_index to file

    if let Ok(web_client_windows) = web_client.windows().await {
        println!("{:?}", web_client_windows);
    }

    web_client.close().await?;
    Ok(())
}

use async_recursion::async_recursion;

#[async_recursion]
async fn create_index(mut url_index: HashMap<String, Url>,mut url_list: HashSet<String>, domains: Vec<String>, web_client: &mut Client) -> Result<HashMap<String, Url>, WebScrapingError> {
    let mut found_urls: HashSet<String> = url_list.clone(); 
    let mut should_return = true;

    let mut url_list_iter = found_urls.into_iter(); 
    println!("Looping through {:?} urls ", url_list_iter.size_hint());
    while let Some(url) = url_list_iter.next() {
        //All urls we iterator through should be found in the index 
        if let Some(url_object) = url_index.get(&url){ 
            // println!("{:?}", url_object);
            //If Url contains Some response code we know we already visited this url
            if let Some(code) = url_object.response_code {
                println!("Found response code: {}", code);
                continue; 
            } else {
                should_return = false;
            }
        }

        let found_urls_vec: Vec<String> = find_all_urls_from_webpage(&url, web_client, domains.clone(), &mut url_index).await?;
        //iterate through all urls and insert to HashSet. 
        let mut found_urls_iter = found_urls_vec.into_iter();
        while let Some(found_url) = found_urls_iter.next() {
            url_list.insert(found_url);
        } 
        
    }
    
    if should_return {
        return Ok(url_index)
    } else {
        return create_index(url_index, url_list, domains, web_client).await
    }
}


async fn is_404(web_client: &mut Client) -> Result<bool, WebScrapingError> {
    let locator = Locator::XPath("//title");

    let mut title = web_client.find(locator).await?; //Vec<Elements>

    let title_text = title.text().await?;

    if title_text.contains("Page Not Found") {
        return Ok(true)
    } else {
        return Ok(false)
    }
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

fn format_urls(mut domain: String, mut urls: Vec<String>) -> Vec<String> {
    let mut urls_iter = urls.iter_mut();

    //remove '/' from end of domain if needed:
    while domain.ends_with("/") {
        domain.pop();
    }
    
    while let Some(url) = urls_iter.next() {
        // Remove # to the end ->
        if let Some(idx) = url.find("#") {
            let (url_replacement, _) = url.split_at(idx);
            
            *url = url_replacement.to_string();
            println!("Url #2: {}", &url);
        }

        if !domain.starts_with("http") {
            //Adds https && http if not included
            let https = String::from("https://");
            let http = String::from("http://");
            if !url.starts_with(&(https.clone() + &domain)) && !url.starts_with(&(http + &domain)) {
                (*url).insert_str(0, &(https + &domain));
            } 
        } else {
            if !url.starts_with(&domain) {
                //add domain to url
                (*url).insert_str(0, &domain);
                println!("New Url: {}", &url);
            }
        }

    }
    urls
}

fn add_to_list(mut urls: Vec<String>, host: String, domain_list: Vec<String>, hash_map: &mut HashMap<String, Url>) -> Result<Vec<String>, WebScrapingError> {
        urls = filter_domains(urls, domain_list);

        //TODO: refactor host.clone() to accept &String
        urls = format_urls(host.clone(), urls);

        let mut url_iter = urls.iter();

        //TODO: refactor host.clone() to accept &String
        while let Some(url_string) = url_iter.next() {
            let url_object = Url::new(url_string.to_string(), None, host.clone());
            if !hash_map.contains_key(&url_string.to_string()) {
                hash_map.insert(url_string.to_string(), url_object);
            }
        }

        Ok(urls)
}
    /// Checks urls to make sure they are in the trusted domains

fn filter_domains(urls: Vec<String>, domain_list: Vec<String>) -> Vec<String> {
    let domain_iter = domain_list.iter();
    urls.into_iter()
        .filter(|url| {
            let mut should_keep = false;

            for domain in domain_iter.clone() {
                if domain.starts_with("http") {
                    if url.starts_with(domain) {
                        should_keep = true;
                        break;
                    } else {
                        continue;
                    }
                }

                //Adds https && http if not included
                let https = String::from("https://");
                let http = String::from("http://");
                if url.starts_with(&(https + domain)) {
                    should_keep = true;
                    break;
                } else if url.starts_with(&(http + domain)) {
                    should_keep = true;
                    break;
                } else if url.starts_with("/") {
                    should_keep = true;
                    break;
                }
            }
            should_keep
        })
        .collect()
}

async fn find_all_urls_from_webpage(url_to_visit: &String, web_client: &mut Client, domain_list: Vec<String>, hash_map: &mut HashMap<String, Url> ) -> Result<Vec<String>, WebScrapingError> {
    web_client.goto(url_to_visit).await?; 

    //set response code on url object: 
    if let Some(url_object) = hash_map.get_mut(url_to_visit) {
        println!("Url Object before change: {:?}", &url_object);
        (*url_object).set_response_code(web_client).await?;
        println!("Url Object after change: {:?}", &url_object);
    } else {
        println!("Hash Map: {:?}", hash_map);
        println!("url to visit: {:}", url_to_visit);
        panic!("Could not find Url Key");
    }

    let locator = Locator::XPath("//a");
    
    let all_urls = find_urls(web_client).await?;
    
    let current_url = web_client.current_url().await?;
    if let Some(host) = current_url.domain() {
        if let Ok(formatted_urls) = add_to_list(all_urls, host.to_string(), domain_list, hash_map) {
            Ok(formatted_urls)
        } else {
            Err(WebScrapingError::FormattingUrlError)
        }
    } else {
        Err(WebScrapingError::FindingDomainError)
    }
    
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     impl PartialEq for UrlIndex {
//         fn eq(&self, other: &UrlIndex) -> bool {
//             let self_bad_urls = self.bad_urls.lock().unwrap();
//             let other_bad_urls = other.bad_urls.lock().unwrap();
//             if *self_bad_urls != *other_bad_urls {
//                 return false;
//             }

//             let self_good_urls = self.good_urls.lock().unwrap();
//             let other_good_urls = other.good_urls.lock().unwrap();
//             if *self_good_urls != *other_good_urls {
//                 return false;
//             }

//             let self_redirected_urls = self.redirected_urls.lock().unwrap();
//             let other_redirected_urls = other.redirected_urls.lock().unwrap();
//             if *self_redirected_urls != *other_redirected_urls {
//                 return false;
//             }

//             let self_error_urls = self.error_urls.lock().unwrap();
//             let other_error_urls = other.error_urls.lock().unwrap();
//             if *self_error_urls != *other_error_urls {
//                 return false;
//             }

//             let self_all_urls = self.all_urls.lock().unwrap();
//             let other_all_urls = other.all_urls.lock().unwrap();
//             if *self_all_urls != *other_all_urls {
//                 return false;
//             }

//             if *self.domain_list != *other.domain_list {
//                 return false;
//             }

//             return true;
//         }
//     }

//     #[test]
//     fn url_new_test() {
//         let url = Url::new(
//             "https://example.com".to_string(),
//             None,
//             "https://google.com/".to_string(),
//         );
//         assert_eq!(
//             url,
//             Url {
//                 url: "https://example.com".to_string(),
//                 response_code: None,
//                 site_references: Arc::new(Mutex::new(vec!["https://google.com/".to_string()])),
//                 redirected_to: None
//             }
//         )
//     }

//     #[test]
//     fn url_near_equal_test_true() {
//         let url = Url::new(
//             "https://example.com".to_string(),
//             None,
//             "https://google.com/".to_string(),
//         );
//         let other = Url::new(
//             "https://example.com".to_string(),
//             None,
//             "https://google.com/123".to_string(),
//         );
//         assert!(url.near_eq(other));
//     }

//     #[test]
//     fn url_near_equal_test_false() {
//         let url = Url::new(
//             "https://example.com".to_string(),
//             None,
//             "https://google.com/".to_string(),
//         );
//         let other = Url::new(
//             "https://example.com/abc".to_string(),
//             None,
//             "https://google.com/123".to_string(),
//         );
//         assert!(!url.near_eq(other));
//     }

//     #[test]
//     fn url_add_reference_test() {
//         let url = Url::new(
//             "https://example.com".to_string(),
//             Some(200),
//             "https://google.com/".to_string(),
//         );
//         url.add_reference("https://facebook.com/".to_string());
//         assert_eq!(
//             url,
//             Url {
//                 url: "https://example.com".to_string(),
//                 response_code: Some(200),
//                 site_references: Arc::new(Mutex::new(vec![
//                     "https://google.com/".to_string(),
//                     "https://facebook.com/".to_string()
//                 ])),
//                 redirected_to: None
//             }
//         )
//     }

//     #[test]
//     fn url_index_new_test() {
//         let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
//         assert_eq!(
//             url_index,
//             UrlIndex {
//                 bad_urls: Arc::new(Mutex::new(Vec::new())),
//                 good_urls: Arc::new(Mutex::new(Vec::new())),
//                 redirected_urls: Arc::new(Mutex::new(Vec::new())),
//                 error_urls: Arc::new(Mutex::new(Vec::new())),
//                 all_urls: Arc::new(Mutex::new(HashSet::new())),
//                 domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
//             }
//         )
//     }

//     #[test]
//     fn url_index_add_good_url_test() {
//         let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
//         let url = Url::new(
//             "https://example.com".to_string(),
//             Some(200),
//             "https://google.com/".to_string(),
//         );
//         let url_copy = url.clone();

//         url_index.add(url);

//         assert_eq!(
//             url_index,
//             UrlIndex {
//                 bad_urls: Arc::new(Mutex::new(Vec::new())),
//                 good_urls: Arc::new(Mutex::new(vec![url_copy])),
//                 redirected_urls: Arc::new(Mutex::new(Vec::new())),
//                 error_urls: Arc::new(Mutex::new(Vec::new())),
//                 all_urls: Arc::new(Mutex::new(HashSet::new())),
//                 domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
//             }
//         )
//     }
//     //Should keep urls on the all_url list while pointing a reference or clone to the good url list
//     #[test]
//     fn url_index_add_good_url_test_stays_on_all_urls() {
//         let url_index = UrlIndex::new(HashSet::from(["https://f3d-shop.forgeflow.io/".to_string()]));
//         url_index.add_to_list(vec!["https://f3d-shop.forgeflow.io/".to_string()], "https://f3d-shop.forgeflow.io/".to_string());
        
//         let all_url_iter_result = filter_out_tested_domains(&url_index);
//         if let Ok(mut url_iter) = all_url_iter_result {        
//             if let Some(mut url) = url_iter.next() {
//                 url.set_response_code(200);
//                 url_index.add(url);
//             } else {
//                 assert!(false);
//             }
//         } else {
//             assert!(false);
//         }
        
//         let url_ghost = Url {
//                     url: "https://f3d-shop.forgeflow.io/".to_string(),
//                     response_code: Some(200),
//                     site_references: Arc::new(Mutex::new(vec!["https://f3d-shop.forgeflow.io/".to_string()])),
//                     redirected_to: None,
//                 };
//         assert_eq!(
//             url_index,
//             UrlIndex {
//                 bad_urls: Arc::new(Mutex::new(Vec::new())),
//                 good_urls: Arc::new(Mutex::new(vec![url_ghost.clone()])),
//                 redirected_urls: Arc::new(Mutex::new(Vec::new())),
//                 error_urls: Arc::new(Mutex::new(Vec::new())),
//                 all_urls: Arc::new(Mutex::new(HashSet::from([url_ghost]))),
//                 domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
//             }
//         )
//     }

//     #[test]
//     fn url_index_add_bad_url_test() {
//         let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
//         let url = Url::new(
//             "https://example.com".to_string(),
//             Some(404),
//             "https://google.com/".to_string(),
//         );
//         let url_copy = url.clone();

//         url_index.add(url);

//         assert_eq!(
//             url_index,
//             UrlIndex {
//                 bad_urls: Arc::new(Mutex::new(vec![url_copy])),
//                 good_urls: Arc::new(Mutex::new(Vec::new())),
//                 redirected_urls: Arc::new(Mutex::new(Vec::new())),
//                 error_urls: Arc::new(Mutex::new(Vec::new())),
//                 all_urls: Arc::new(Mutex::new(HashSet::new())),
//                 domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
//             }
//         )
//     }

//     #[test]
//     fn url_index_add_redirected_url_test() {
//         let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
//         let url = Url::new(
//             "https://example.com".to_string(),
//             Some(301),
//             "https://google.com/".to_string(),
//         );
//         let url_copy = url.clone();

//         url_index.add(url);

//         assert_eq!(
//             url_index,
//             UrlIndex {
//                 bad_urls: Arc::new(Mutex::new(Vec::new())),
//                 good_urls: Arc::new(Mutex::new(Vec::new())),
//                 redirected_urls: Arc::new(Mutex::new(vec![url_copy])),
//                 error_urls: Arc::new(Mutex::new(Vec::new())),
//                 all_urls: Arc::new(Mutex::new(HashSet::new())),
//                 domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
//             }
//         )
//     }

//     #[test]
//     fn url_index_add_error_url_test() {
//         let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
//         let url = Url::new(
//             "https://example.com".to_string(),
//             Some(500),
//             "https://google.com/".to_string(),
//         );
//         let url_copy = url.clone();

//         url_index.add(url);

//         assert_eq!(
//             url_index,
//             UrlIndex {
//                 bad_urls: Arc::new(Mutex::new(Vec::new())),
//                 good_urls: Arc::new(Mutex::new(Vec::new())),
//                 redirected_urls: Arc::new(Mutex::new(Vec::new())),
//                 error_urls: Arc::new(Mutex::new(vec![url_copy])),
//                 all_urls: Arc::new(Mutex::new(HashSet::from([]))),
//                 domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
//             }
//         )
//     }

//     #[test]
//     fn url_index_add_one_all_url_test() {
//         let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
//         let url = vec!["https://example.com".to_string()];

//         url_index.add_to_list(url, "https://example.com".to_string());

//         let mut hash_set = HashSet::new();
//         hash_set.insert(Url::new(
//             "https://example.com".to_string(),
//             None,
//             "https://example.com".to_string(),
//         ));

//         assert_eq!(
//             url_index,
//             UrlIndex {
//                 bad_urls: Arc::new(Mutex::new(Vec::new())),
//                 good_urls: Arc::new(Mutex::new(Vec::new())),
//                 redirected_urls: Arc::new(Mutex::new(Vec::new())),
//                 error_urls: Arc::new(Mutex::new(Vec::new())),
//                 all_urls: Arc::new(Mutex::new(hash_set)),
//                 domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
//             }
//         )
//     }

//     #[test]
//     fn url_index_add_multiple_all_url_test() {
//         let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
//         let url = vec![
//             "https://example.com".to_string(),
//             "https://example.com/123".to_string(),
//             "https://example.com/abc".to_string(),
//         ];

//         url_index.add_to_list(url, "https://example.com".to_string());

//         let mut hash_set = HashSet::new();
//         hash_set.insert(Url::new(
//             "https://example.com".to_string(),
//             None,
//             "https://example.com".to_string(),
//         ));
//         hash_set.insert(Url::new(
//             "https://example.com/123".to_string(),
//             None,
//             "https://example.com".to_string(),
//         ));
//         hash_set.insert(Url::new(
//             "https://example.com/abc".to_string(),
//             None,
//             "https://example.com".to_string(),
//         ));

//         assert_eq!(
//             url_index,
//             UrlIndex {
//                 bad_urls: Arc::new(Mutex::new(Vec::new())),
//                 good_urls: Arc::new(Mutex::new(Vec::new())),
//                 redirected_urls: Arc::new(Mutex::new(Vec::new())),
//                 error_urls: Arc::new(Mutex::new(Vec::new())),
//                 all_urls: Arc::new(Mutex::new(hash_set)),
//                 domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
//             }
//         )
//     }

//     #[test]
//     fn url_index_add_test_avoid_duplicates() {
//         let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
//         let url = vec![
//             "https://example.com".to_string(),
//             "https://example.com/123".to_string(),
//             "https://example.com/abc".to_string(),
//             "https://example.com/123".to_string(),
//             "https://example.com/abc".to_string(),
//         ];

//         url_index.add_to_list(url, "https://example.com".to_string());

//         let mut hash_set = HashSet::new();
//         hash_set.insert(Url::new(
//             "https://example.com".to_string(),
//             None,
//             "https://example.com".to_string(),
//         ));
//         hash_set.insert(Url::new(
//             "https://example.com/123".to_string(),
//             None,
//             "https://example.com".to_string(),
//         ));
//         hash_set.insert(Url::new(
//             "https://example.com/abc".to_string(),
//             None,
//             "https://example.com".to_string(),
//         ));

//         assert_eq!(
//             url_index,
//             UrlIndex {
//                 bad_urls: Arc::new(Mutex::new(Vec::new())),
//                 good_urls: Arc::new(Mutex::new(Vec::new())),
//                 redirected_urls: Arc::new(Mutex::new(Vec::new())),
//                 error_urls: Arc::new(Mutex::new(Vec::new())),
//                 all_urls: Arc::new(Mutex::new(hash_set)),
//                 domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
//             }
//         )
//     }

//     #[test]
//     fn format_urls_test() {
//         let urls: Vec<String> = vec![
//             "#pop-up".to_string(),
//             "/about-me".to_string(),
//             "/support?search=3d+printers".to_string(),
//             "https://lulzbot.com/3d-printers/".to_string(),
//         ];

//         let domain = "https://lulzbot.com/".to_string();

//         assert_eq!(
//             format_urls(domain, urls),
//             vec![
//                 "https://lulzbot.com".to_string(),
//                 "https://lulzbot.com/about-me".to_string(),
//                 "https://lulzbot.com/support?search=3d+printers".to_string(),
//                 "https://lulzbot.com/3d-printers/".to_string()
//             ]
//         );
//     }

//     #[test]
//     fn format_urls_test_no_https() {
//         let urls: Vec<String> = vec![
//             "#pop-up".to_string(),
//             "/about-me".to_string(),
//             "/support?search=3d+printers".to_string(),
//             "https://lulzbot.com/3d-printers/".to_string(),
//         ];

//         let domain = "lulzbot.com/".to_string();

//         assert_eq!(
//             format_urls(domain, urls),
//             vec![
//                 "https://lulzbot.com".to_string(),
//                 "https://lulzbot.com/about-me".to_string(),
//                 "https://lulzbot.com/support?search=3d+printers".to_string(),
//                 "https://lulzbot.com/3d-printers/".to_string()
//             ]
//         );
//     }

//     #[test]
//     fn filter_domains_test() {
//         let domains = HashSet::from([
//             "lulzbot.com".to_string(),
//             "www.lulzbot.com".to_string(),
//             "shop.lulzbot.com".to_string(),
//             "learn.lulzbot.com".to_string(),
//         ]);
//         let url_index = UrlIndex::new(domains);

//         let urls: Vec<String> = vec![
//             "https://lulzbot.com/3d-printers/".to_string(),
//             "https://makerbot.com/3d-printers/".to_string(),
//             "https://shop.lulzbot.com/3d-printers/".to_string(),
//             "http://learn.lulzbot.com/learn/".to_string(),
//             "/learn/here".to_string(),
//         ];

//         assert_eq!(
//             url_index.filter_domains(urls),
//             vec![
//                 "https://lulzbot.com/3d-printers/".to_string(),
//                 "https://shop.lulzbot.com/3d-printers/".to_string(),
//                 "http://learn.lulzbot.com/learn/".to_string(),
//                 "/learn/here".to_string(),
//             ]
//         );
//     }

//     #[test]
//     fn filter_domains_test_limit_domains() {
//         let domains = HashSet::from([
//             "lulzbot.com".to_string(),
//             "www.lulzbot.com".to_string(),
//             "shop.lulzbot.com".to_string(),
//             "learn.lulzbot.com".to_string(),
//         ]);
//         let url_index = UrlIndex::new(HashSet::from(domains));

//         let urls: Vec<String> = vec![
//             "https://lulzbot.com/3d-printers/".to_string(),
//             "https://makerbot.com/3d-printers/".to_string(),
//             "https://shop.lulzbot.com/3d-printers/".to_string(),
//             "http://learn.lulzbot.com/learn/".to_string(),
//             "http://forum.lulzbot.com/learn/".to_string(),
//             "/learn/here".to_string(),
//         ];

//         assert_eq!(
//             url_index.filter_domains(urls),
//             vec![
//                 "https://lulzbot.com/3d-printers/".to_string(),
//                 "https://shop.lulzbot.com/3d-printers/".to_string(),
//                 "http://learn.lulzbot.com/learn/".to_string(),
//                 "/learn/here".to_string(),
//             ]
//         );
//     }

//     #[test]
//     fn url_add_redirection_test() {
//         let mut url = Url::new(
//             "https://example.com/base".to_string(),
//             Some(301),
//             "https://example.com".to_string(),
//         );
//         let destination = String::from("https://example.com/redirected");

//         url.set_redirection(destination);

//         assert_eq!(
//             url,
//             Url {
//                 url: String::from("https://example.com/base"),
//                 response_code: Some(301),
//                 site_references: Arc::new(Mutex::new(vec!["https://example.com".to_string()])),
//                 redirected_to: Some(String::from("https://example.com/redirected"))
//             }
//         )
//     }

//     #[test]
//     fn filter_out_tested_domains_test() {
       
//         let mut hash_set = HashSet::new();
//         hash_set.insert(Url::new(
//             "https://example.com".to_string(),
//             None,
//             "https://example.com".to_string(),
//         ));
//         hash_set.insert(Url::new(
//             "https://example.com/123".to_string(),
//             Some(200),
//             "https://example.com".to_string(),
//         ));
//         hash_set.insert(Url::new(
//             "https://example.com/abc".to_string(),
//             None,
//             "https://example.com".to_string(),
//         ));
//         hash_set.insert(Url::new(
//             "https://example.com/def".to_string(),
//             None,
//             "https://example.com".to_string(),
//         ));
//         hash_set.insert(Url::new(
//             "https://example.com/hij".to_string(),
//             Some(500),
//             "https://example.com".to_string(),
//         ));
       
//         let url_index = UrlIndex {
//                 bad_urls: Arc::new(Mutex::new(Vec::new())),
//                 good_urls: Arc::new(Mutex::new(Vec::new())),
//                 redirected_urls: Arc::new(Mutex::new(Vec::new())),
//                 error_urls: Arc::new(Mutex::new(Vec::new())),
//                 all_urls: Arc::new(Mutex::new(hash_set)),
//                 domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
//             };
//         if let Ok(mut result) = filter_out_tested_domains(&url_index) {
//             while let Some(url) = result.next() {
//                 if url.response_code != None {
//                     assert!(false)
//                 }
//             }
//         } else {
//             assert!(false)
//         }
//     }
    
//     #[test]
//     fn filter_out_tested_domains_test_final() {
       
//         let mut hash_set = HashSet::new();
//         hash_set.insert(Url::new(
//             "https://example.com/123".to_string(),
//             Some(200),
//             "https://example.com".to_string(),
//         ));
//         hash_set.insert(Url::new(
//             "https://example.com/hij".to_string(),
//             Some(500),
//             "https://example.com".to_string(),
//         ));
       
//         let url_index = UrlIndex {
//                 bad_urls: Arc::new(Mutex::new(Vec::new())),
//                 good_urls: Arc::new(Mutex::new(Vec::new())),
//                 redirected_urls: Arc::new(Mutex::new(Vec::new())),
//                 error_urls: Arc::new(Mutex::new(Vec::new())),
//                 all_urls: Arc::new(Mutex::new(hash_set)),
//                 domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
//             };

//         if let Ok(mut result) = filter_out_tested_domains(&url_index) {
//             // Should return an empty iterator. 
//             assert_eq!(result.next(), None)
//         } else {
//             assert!(false)
//         }
//     }

//     #[test]
//     fn url_set_response_code_test() {
//        let mut url = Url::new(
//             "https://example.com".to_string(),
//             None,
//             "https://google.com/".to_string(),
//         );
        
//         url.set_response_code(404);

//         assert_eq!(
//             url,
//             Url {
//                 url: "https://example.com".to_string(),
//                 response_code: Some(404),
//                 site_references: Arc::new(Mutex::new(vec!["https://google.com/".to_string()])),
//                 redirected_to: None
//             }
//         )
//     }

//     // #[test]
//     // fn url_index_write_to_file_test() {
//     //     let good_url1 = Url {
//     //         url: "https://example.com/good-urls".to_string(),
//     //         response_code: Some(200),
//     //         site_references: Arc::new(Mutex::new(vec!["https://example.com".to_string()])),
//     //         redirected_to: None
//     //     };
//     //     let good_url2 = Url {
//     //         url: "https://example.com/good-urls?q=how%20do%20i%20know".to_string(),
//     //         response_code: Some(200),
//     //         site_references: Arc::new(Mutex::new(vec!["https://example.com".to_string()])),
//     //         redirected_to: None
//     //     };
//     //     let bad_url1 = Url {
//     //         url: "https://example.com/bad-urls".to_string(),
//     //         response_code: Some(404),
//     //         site_references: Arc::new(Mutex::new(vec!["https://example.com".to_string()])),
//     //         redirected_to: None
//     //     };
//     //     let bad_url2 = Url {
//     //         url: "https://example.com/bad-urls?q=how%20do%20i%20know".to_string(),
//     //         response_code: Some(404),
//     //         site_references: Arc::new(Mutex::new(vec!["https://example.com".to_string()])),
//     //         redirected_to: None
//     //     };
//     //     let redirected_url1 = Url {
//     //         url: "https://example.com/redirected-urls".to_string(),
//     //         response_code: Some(300),
//     //         site_references: Arc::new(Mutex::new(vec!["https://example.com".to_string()])),
//     //         redirected_to: Some("https://example.com/redirected-here".to_string())
//     //     };
//     //     let redirected_url2 = Url {
//     //         url: "https://example.com/redirected-urls?q=how%20do%20i%20know".to_string(),
//     //         response_code: Some(300),
//     //         site_references: Arc::new(Mutex::new(vec!["https://example.com".to_string()])),
//     //         redirected_to: Some("https://example.com/redirected-here".to_string())
//     //     };
//     //     let error_url1 = Url {
//     //         url: "https://example.com/error-urls".to_string(),
//     //         response_code: Some(500),
//     //         site_references: Arc::new(Mutex::new(vec!["https://example.com".to_string()])),
//     //         redirected_to: None
//     //     };
//     //     let error_url2 = Url {
//     //         url: "https://example.com/error-urls?q=how%20do%20i%20know".to_string(),
//     //         response_code: Some(500),
//     //         site_references: Arc::new(Mutex::new(vec!["https://example.com".to_string()])),
//     //         redirected_to: None
//     //     };
//     //     let hash_set = HashSet::from([good_url1, good_url2, bad_url1, bad_url2, redirected_url1, redirected_url2, error_url1, error_url2]);

//     //     let url_index = UrlIndex {
//     //             bad_urls: Arc::new(Mutex::new(vec![bad_url1, bad_url2])),
//     //             good_urls: Arc::new(Mutex::new(vec![good_url1, good_url2])),
//     //             redirected_urls: Arc::new(Mutex::new(vec![redirected_url1, redirected_url2])),
//     //             error_urls: Arc::new(Mutex::new(vec![error_url1, error_url2])),
//     //             all_urls: Arc::new(Mutex::new(hash_set)),
//     //             domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
//     //         };
//     // } 
// }
