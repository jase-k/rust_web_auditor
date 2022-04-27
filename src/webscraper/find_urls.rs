use crate::webdriver::webdriver::{DriverHandle, WebDriver};
use async_recursion::async_recursion;
use fantoccini::elements::Element;
use fantoccini::error::{CmdError, NewSessionError};
use fantoccini::{Client, ClientBuilder, Locator};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::Path;
use reqwest::{ClientBuilder as ReqwestClientBuilder, Client as ReqwestClient};
use futures::future::join_all;
use std::time::Duration;

#[derive(Debug)]
pub enum WebScrapingError {
    FantocciniNewSessionError(NewSessionError),
    FantocciniCmdErrorr(CmdError),
    FormattingUrlError,
    WritingToFileError,
    ProblemIndexingExternals,
    ResponseCode500
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

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
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
    //TODO combine these two functions into one by checking response codes after indexing urls
    async fn set_response_code(&mut self, web_client: &mut Client, not_found_title: &String) -> Result<(), WebScrapingError> {
        let current_url = web_client.current_url().await?;

        if is_404(web_client, not_found_title).await? {
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

    fn set_response_code_(&mut self, code: u16) -> &Self {
        self.response_code = Some(code);
        self 
    }
}

/// Public function
pub async fn index_urls(
    starting_url: String,
    domains: Vec<String>,
    not_found_title: String
) -> Result<(), WebScrapingError> {
    //Launches WebDriver
    let mut webdriver: DriverHandle = DriverHandle::new(WebDriver::GeckoDriver);

    let first_url = Url::new(starting_url.clone(), None, starting_url.clone());

    let url_hash_set: HashSet<String> = HashSet::from([starting_url.clone()]);

    let url_index: HashMap<String, Url> = HashMap::from([(starting_url, first_url)]);

    let external_urls: HashMap<String, Url> = HashMap::new();

    println!("Opening Up Web Client");
    let mut web_client: Client = open_new_client().await?;
    println!("Connected to Web Client");

    let (internal_index, mut external_index) =
        create_index(url_index, url_hash_set, domains, &mut web_client, &not_found_title, external_urls).await?;
    
    println!("Closing to Web Client");
    web_client.close().await?;
    println!("Closed to Web Client");

    //Exits Gecko-Driver
    if let Err(e) = webdriver.kill() {
        println!("Error closing Webdriver: {:?}", e);
    }

    // Check for external 404s //
    println!("Checking External Link Statuses: ");

    if let Ok(updated_external_index) = check_external_urls(&mut external_index).await {
        write_to_file(updated_external_index, "./data/external_urls.json")?;
    } else {
        println!("Warning: Error while indexing external urls");
        write_to_file(external_index, "./data/external_urls.json")?;
    }


    write_to_file(internal_index, "./data/internal_urls.json")?;



    Ok(())
}

async fn get_url_information(request_client: ReqwestClient, url: &mut Url) -> Result<(String, Url), WebScrapingError> {
    println!("Checking: {}", &url.full_path);
    //Set a limit for this send
    let response_result = request_client.get(url.full_path.clone()).send().await;
    
    println!("Recieved Response from: {}", &url.full_path);

    if let Ok(res) = response_result {
        let response_code = res.status().as_u16();
        let mut new_url = url.clone();
        if response_code > 299 && response_code < 400 {
            new_url.set_redirection(res.url().as_str().to_string());
        }

        new_url.set_response_code_(response_code);


        return Ok((url.full_path.clone(), new_url))
    } else {
        Err(WebScrapingError::ProblemIndexingExternals)
    }
}

async fn check_external_urls(url_list: &mut HashMap<String, Url>) -> Result<HashMap<String, Url>, WebScrapingError> {
    let mut url_list_iter = url_list.into_iter();
    println!("Found {:?} of external Urls", url_list_iter.size_hint());

    let web_client_result = ReqwestClientBuilder::new().timeout(Duration::from_secs(30)).build();
    if let Ok(web_client) = web_client_result {
        
        let mut futures = vec![];
    
        //creates a vector of futures to join and run below: 
        while let Some((_, url)) = url_list_iter.next() {
            futures.push(get_url_information(web_client.clone(), url))
        }
    
        let future_results = join_all(futures).await;
        let mut future_results_iter = future_results.into_iter();
    
        let mut new_hashmap = HashMap::new();
        
        println!("Building JSON");
        while let Some(result) = future_results_iter.next() {
            if let Ok((key, value)) = result {
                new_hashmap.insert(key, value);
            }
        }
    
        Ok(new_hashmap)
    } else {
        Err(WebScrapingError::ProblemIndexingExternals)
    }
}

/// Print to file data/all_urls.json
fn write_to_file(internal_urls: HashMap<String, Url>, file_path: &str) -> Result<(), WebScrapingError> {
    if let Err(_) = fs::DirBuilder::new().recursive(true).create("./data") {
        println!("Trouble creating data directory!");
        return Err(WebScrapingError::WritingToFileError);
    }
    if let Ok(mut good_urls_file) = fs::File::options()
        .write(true)
        .create(true)
        .open(Path::new(file_path))
    {
        if let Ok(string) = serde_json::to_string(&internal_urls) {
            if let Ok(_) = good_urls_file.write(string.as_bytes()) {
                Ok(())
            } else {
                println!("Trouble writing data!");
                Err(WebScrapingError::WritingToFileError)
            }
        } else {
            println!("Trouble Parsing data!");
            Err(WebScrapingError::WritingToFileError)
        }
    } else {
        println!("Trouble Opening File!");
        Err(WebScrapingError::WritingToFileError)
    }
}

#[async_recursion]
async fn create_index(
    mut url_index: HashMap<String, Url>,
    mut url_list: HashSet<String>,
    domains: Vec<String>,
    web_client: &mut Client,
    not_found_title: &String,
    mut external_urls: HashMap<String, Url>
) -> Result<(HashMap<String, Url> /* Internal urls*/, HashMap<String, Url> /* external urls*/), WebScrapingError> {
    let found_urls: HashSet<String> = url_list.clone();
    let mut should_return = true;

    let mut url_list_iter = found_urls.into_iter();
    println!("Looping through {:?} urls ", url_list_iter.size_hint());
    while let Some(url) = url_list_iter.next() {
        //All urls we iterator through should be found in the index
        if let Some(url_object) = url_index.get(&url) {
            // println!("{:?}", url_object);
            //If Url contains Some response code we know we already visited this url
            if let Some(_) = url_object.response_code {
                continue;
            } else {
                should_return = false;
            }
        }

        let found_urls_vec_result =
            find_all_urls_from_webpage(&url, web_client, domains.clone(), &mut url_index, &mut external_urls, &not_found_title).await;
        //iterate through all urls and insert to HashSet.
        if let Ok(found_urls_vec) = found_urls_vec_result{
            let mut found_urls_iter = found_urls_vec.into_iter();
            while let Some(found_url) = found_urls_iter.next() {
                url_list.insert(found_url);
            }
        } else {
            println!("Error finding urls from {}", &url);
        }
    }

    if should_return {
        return Ok((url_index, external_urls));
    } else {
        return create_index(url_index, url_list, domains, web_client, &not_found_title, external_urls).await;
    }
}

async fn is_404(web_client: &mut Client, not_found_title: &String) -> Result<bool, WebScrapingError> {
    let locator = Locator::XPath("//title");

    let mut title = web_client.find(locator).await?; //Element

    let title_text = title.html(true).await?;

    if title_text.to_lowercase().contains(&not_found_title.to_lowercase()) {
        return Ok(true);
    } else {
        return Ok(false);
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

    // remove '/' from end of domain if needed:
    while domain.ends_with("/") {
        domain.pop();
    }

    while let Some(url) = urls_iter.next() {
        // Remove # to the end ->
        if let Some(idx) = url.find("#") {
            let (url_replacement, _) = url.split_at(idx);

            *url = url_replacement.to_string();
        }
        *url = url.as_str().trim().to_string();

        if url.starts_with("http") {
            continue;
        }

        if !domain.starts_with("http") {
            //Adds https && http if not included
            let https = String::from("https://");
            let http = String::from("http://");
            if !url.starts_with(&(https.clone() + &domain)) && !url.starts_with(&(http + &domain)) {
                if !url.starts_with("/") && !url.starts_with("?") && !url.starts_with("#") && url.len() > 0 {
                    (*url).insert(0, '/');
                };
                (*url).insert_str(0, &(https + &domain));
            }
        } else {
            if !url.starts_with(&domain) {
                if !url.starts_with("/") && !url.starts_with("?") && !url.starts_with("#") && url.len() > 0 {
                    (*url).insert(0, '/');
                };
                
                //add domain to url
                (*url).insert_str(0, &domain);
            }
        }
    }
    urls
}

fn add_to_list(
    urls: Vec<String>,
    host: String,
    domain_list: Vec<String>,
    url_index: &mut HashMap<String, Url>,
    external_url_index: &mut HashMap<String, Url>,
    current_domain: String,
) -> Result<Vec<String>, WebScrapingError> {
    let (mut internal_urls, external_urls) = filter_domains(urls, domain_list);

    internal_urls = format_urls(current_domain, internal_urls);

    let mut internal_url_iter = internal_urls.iter();

    while let Some(url_string) = internal_url_iter.next() {
        if !url_index.contains_key(&url_string.to_string()) {
            let url_object = Url::new(url_string.to_string(), None, host.clone());
            url_index.insert(url_string.to_string(), url_object);
        } else {
            if let Some(url_object) = url_index.get_mut(&url_string.to_string()) {
                (*url_object).add_reference(host.clone());
            } else {
                panic!("Could not find Url Key");
            }
        }
    }

    if let Err(_) = index_external_urls(external_urls, external_url_index, host) {
        println!("Warning: Problem Indexing External Urls");
    }

    
    Ok(internal_urls)
}

fn index_external_urls(external_urls: Vec<String>, external_url_index: &mut HashMap<String, Url>, current_url: String) -> Result<(), WebScrapingError> {
    let mut urls_iter = external_urls.iter();
    
    while let Some(url_string) = urls_iter.next() {
        if !external_url_index.contains_key(&url_string.to_string()) {
            let url_object = Url::new(url_string.to_string(), None, current_url.clone());
            external_url_index.insert(url_string.to_string(), url_object);
        } else {
            if let Some(url_object) = external_url_index.get_mut(&url_string.to_string()) {
                (*url_object).add_reference(current_url.clone());
            } else {
                panic!("Could not find Url Key");
            }
        }
    }
    Ok(())
}

/// Checks urls to make sure they are in the trusted domains
fn filter_domains(urls: Vec<String>, domain_list: Vec<String>) -> (Vec<String>, Vec<String>) { //internal urls, external urls// 
    let mut url_iter = urls.iter();
    let mut external_urls: Vec<String> = Vec::new();
    let mut internal_urls: Vec<String> = Vec::new();

    while let Some(url) = url_iter.next() {
        if is_internal(url.to_string(), &domain_list) {
            internal_urls.push(url.to_string());
        } else {
            external_urls.push(url.to_string());
        }
    }
    (internal_urls, external_urls)
}

fn is_internal(url: String, domains: &Vec<String>) -> bool {
    let domain_iter = domains.iter();

    let mut is_internal = false;

    if !url.starts_with("http") {
        return true
    }

    for domain in domain_iter {
        if domain.starts_with("http") {
            if url.starts_with(domain) {
                is_internal = true;
                break;
            } else {
                continue;
            }
        }

        //Adds https && http if not included
        let https = String::from("https://");
        let http = String::from("http://");
        if url.starts_with(&(https + domain)) {
            is_internal = true;
            break;
        } else if url.starts_with(&(http + domain)) {
            is_internal = true;
            break;
        } 
        // else if url.starts_with("/") || url.starts_with("?") {
        //     is_internal = true;
        //     break;
        // }
    }

    is_internal
}

async fn find_all_urls_from_webpage(
    url_to_visit: &String,
    web_client: &mut Client,
    domain_list: Vec<String>,
    url_index: &mut HashMap<String, Url>,
    external_urls: &mut HashMap<String, Url>,
    not_found_title: &String
) -> Result<Vec<String>, WebScrapingError> {
    if let Err(_) = web_client.goto(url_to_visit).await {
        if let Some(url) = url_index.get_mut(url_to_visit){
            url.set_response_code_(500);
        } else {
            panic!("Could not set 500 code on url will result in infinite loop!");
        }

        return Err(WebScrapingError::ResponseCode500)

    }

    //set response code on url object:
    if let Some(url_object) = url_index.get_mut(url_to_visit) {
        (*url_object).set_response_code(web_client, not_found_title).await?;
    } else {
        panic!("Could not find Url Key");
    }

    let all_urls = find_urls(web_client).await?;

    let current_url = web_client.current_url().await?;
    if let Some(current_domain) = current_url.domain() {
        if let Ok(formatted_urls) = add_to_list(
            all_urls,
            current_url.as_str().to_string(),
            domain_list,
            url_index,
            external_urls,
            current_domain.to_string(),
        ) {
            Ok(formatted_urls)
        } else {
            Err(WebScrapingError::FormattingUrlError)
        }
    } else {
        Err(WebScrapingError::FormattingUrlError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_new_test() {
        let url = Url::new(
            "https://example.com".to_string(),
            None,
            "https://google.com/".to_string(),
        );
        assert_eq!(
            url,
            Url {
                full_path: "https://example.com".to_string(),
                response_code: None,
                site_references: vec!["https://google.com/".to_string()],
                redirected_to: None
            }
        )
    }

    #[test]
    fn format_urls_test() {
        let urls: Vec<String> = vec![
            "#pop-up".to_string(),
            "/about-me".to_string(),
            "/support?search=3d+printers".to_string(),
            " https://lulzbot.com/3d-printers/".to_string(),
        ];

        let domain = "https://lulzbot.com".to_string();

        assert_eq!(
            format_urls(domain, urls),
            vec![
                "https://lulzbot.com".to_string(),
                "https://lulzbot.com/about-me".to_string(),
                "https://lulzbot.com/support?search=3d+printers".to_string(),
                "https://lulzbot.com/3d-printers/".to_string()
            ]
        );
    }

    #[test]
    fn format_urls_test_no_https() {
        let urls: Vec<String> = vec![
            "#pop-up".to_string(),
            "/about-me".to_string(),
            "/support?search=3d+printers".to_string(),
            "https://lulzbot.com/3d-printers/".to_string(),
        ];

        let domain = "lulzbot.com".to_string();

        assert_eq!(
            format_urls(domain, urls),
            vec![
                "https://lulzbot.com".to_string(),
                "https://lulzbot.com/about-me".to_string(),
                "https://lulzbot.com/support?search=3d+printers".to_string(),
                "https://lulzbot.com/3d-printers/".to_string()
            ]
        );
    }

    #[test]
    fn format_urls_test_no_slash() {
        let urls: Vec<String> = vec![
            "#pop-up".to_string(),
            "about-me".to_string(),
            "?search=3d+printers".to_string(),
            "https://learn.lulzbot.com/support/cura".to_string(),
            ];
            
        let domain = "lulzbot.com/".to_string();
        
        assert_eq!(
            format_urls(domain, urls),
            vec![
                "https://lulzbot.com".to_string(),
                "https://lulzbot.com/about-me".to_string(),
                "https://lulzbot.com?search=3d+printers".to_string(),
                "https://learn.lulzbot.com/support/cura".to_string(),
            ]
        );
    }

    #[test]
    fn filter_domains_test() {
        let domains = vec![
            "lulzbot.com".to_string(),
            "www.lulzbot.com".to_string(),
            "shop.lulzbot.com".to_string(),
            "learn.lulzbot.com".to_string(),
        ];

        let urls: Vec<String> = vec![
            "https://lulzbot.com/3d-printers/".to_string(),
            "https://makerbot.com/3d-printers/".to_string(),
            "https://shop.lulzbot.com/3d-printers/".to_string(),
            " http://learn.lulzbot.com/learn/".to_string(),
            "/learn/here".to_string(),
            ];
            
            assert_eq!(
                filter_domains(urls, domains),
                (
                    vec![
                        "https://lulzbot.com/3d-printers/".to_string(),
                        "https://shop.lulzbot.com/3d-printers/".to_string(),
                        " http://learn.lulzbot.com/learn/".to_string(),
                        "/learn/here".to_string(),
                        ],
                    vec![
                    "https://makerbot.com/3d-printers/".to_string(),
                    ]
                )
        );
    }

    #[test]
    fn filter_domains_test_question_mark() {
        let domains = vec![
            "lulzbot.com".to_string(),
            "www.lulzbot.com".to_string(),
            "shop.lulzbot.com".to_string(),
            "learn.lulzbot.com".to_string(),
        ];

        let urls: Vec<String> = vec![
            "https://lulzbot.com/3d-printers/".to_string(),
            "https://makerbot.com/3d-printers/".to_string(),
            "https://shop.lulzbot.com/3d-printers/".to_string(),
            "http://learn.lulzbot.com/learn/".to_string(),
            "?topic=Problem+Solving".to_string(),
            ];
            
            assert_eq!(
                filter_domains(urls, domains),
                ( vec![
                    "https://lulzbot.com/3d-printers/".to_string(),
                    "https://shop.lulzbot.com/3d-printers/".to_string(),
                    "http://learn.lulzbot.com/learn/".to_string(),
                    "?topic=Problem+Solving".to_string(),
                    ], 
                    vec![
                        "https://makerbot.com/3d-printers/".to_string(),
            ]
            )
        );
    }

    #[test]
    fn filter_domains_test_limit_domains() {
        let domains = vec![
            "lulzbot.com".to_string(),
            "www.lulzbot.com".to_string(),
            "shop.lulzbot.com".to_string(),
            "learn.lulzbot.com".to_string(),
        ];

        let urls: Vec<String> = vec![
            "https://lulzbot.com/3d-printers/".to_string(),
            "https://makerbot.com/3d-printers/".to_string(),
            "https://shop.lulzbot.com/3d-printers/".to_string(),
            "http://learn.lulzbot.com/learn/".to_string(),
            "http://forum.lulzbot.com/learn/".to_string(),
            "/learn/here".to_string(),
            ];
            
            assert_eq!(
                filter_domains(urls, domains),
                (vec![
                    "https://lulzbot.com/3d-printers/".to_string(),
                    "https://shop.lulzbot.com/3d-printers/".to_string(),
                    "http://learn.lulzbot.com/learn/".to_string(),
                    "/learn/here".to_string(),
                    ],
                    vec![
                        "https://makerbot.com/3d-printers/".to_string(),
                        "http://forum.lulzbot.com/learn/".to_string(),
                    ]
        )
        );
    }

    #[test]
    fn is_internal_test() {
        // fn is_internal(url: String, domains: &Vec<String>) -> bool 
        let domains = vec!["example.com".to_string(), "www.example.com".to_string()];

        assert!(is_internal("https://example.com/abs".to_string(), &domains));
        assert!(is_internal("/abs".to_string(), &domains));
        assert!(is_internal("?abs".to_string(), &domains));
        assert!(is_internal("abs".to_string(), &domains));
    }

    #[test]
    fn url_set_redirection_test() {
        let mut url = Url::new(
            "https://example.com/base".to_string(),
            Some(301),
            "https://example.com".to_string(),
        );
        let destination = String::from("https://example.com/redirected");

        url.set_redirection(destination);

        assert_eq!(
            url,
            Url {
                full_path: String::from("https://example.com/base"),
                response_code: Some(301),
                site_references: vec!["https://example.com".to_string()],
                redirected_to: Some(String::from("https://example.com/redirected"))
            }
        )
    }

    #[test]
    fn url_add_reference_test() {
        let mut url = Url::new(
            "https://example.com/base".to_string(),
            Some(301),
            "https://example.com".to_string(),
        );
        let destination = String::from("https://example.com/redirected");

        url.add_reference(destination.clone());

        assert_eq!(
            url,
            Url {
                full_path: String::from("https://example.com/base"),
                response_code: Some(301),
                site_references: vec!["https://example.com".to_string(), destination.to_string()],
                redirected_to: None
            }
        )
    }
}
