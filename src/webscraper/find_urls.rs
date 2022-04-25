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

#[derive(Debug)]
pub enum WebScrapingError {
    FantocciniNewSessionError(NewSessionError),
    FantocciniCmdErrorr(CmdError),
    FormattingUrlError,
    WritingToFileError,
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

    println!("Opening Up Web Client");
    let mut web_client: Client = open_new_client().await?;
    println!("Connected to Web Client");

    let final_index: HashMap<String, Url> =
        create_index(url_index, url_hash_set, domains, &mut web_client, &not_found_title).await?;

    write_to_file(final_index)?;

    println!("Closing to Web Client");
    web_client.close().await?;
    println!("Closed to Web Client");

    //Exits Gecko-Driver
    if let Err(e) = webdriver.kill() {
        println!("Error closing Webdriver: {:?}", e);
    }

    Ok(())
}

/// Print to file data/all_urls.json
fn write_to_file(hash_map: HashMap<String, Url>) -> Result<(), WebScrapingError> {
    if let Err(_) = fs::DirBuilder::new().recursive(true).create("./data") {
        println!("Trouble creating data directory!");
        return Err(WebScrapingError::WritingToFileError);
    }
    if let Ok(mut good_urls_file) = fs::File::options()
        .write(true)
        .create(true)
        .open(Path::new("./data/all_urls.json"))
    {
        if let Ok(string) = serde_json::to_string(&hash_map) {
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
    not_found_title: &String
) -> Result<HashMap<String, Url>, WebScrapingError> {
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

        let found_urls_vec: Vec<String> =
            find_all_urls_from_webpage(&url, web_client, domains.clone(), &mut url_index, &not_found_title).await?;
        //iterate through all urls and insert to HashSet.
        let mut found_urls_iter = found_urls_vec.into_iter();
        while let Some(found_url) = found_urls_iter.next() {
            url_list.insert(found_url);
        }
    }

    if should_return {
        return Ok(url_index);
    } else {
        return create_index(url_index, url_list, domains, web_client, &not_found_title).await;
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

fn add_to_list(
    mut urls: Vec<String>,
    host: String,
    domain_list: Vec<String>,
    hash_map: &mut HashMap<String, Url>,
    current_domain: String,
) -> Result<Vec<String>, WebScrapingError> {
    urls = filter_domains(urls, domain_list);

    urls = format_urls(current_domain, urls);

    let mut url_iter = urls.iter();

    while let Some(url_string) = url_iter.next() {
        if !hash_map.contains_key(&url_string.to_string()) {
            let url_object = Url::new(url_string.to_string(), None, host.clone());
            hash_map.insert(url_string.to_string(), url_object);
        } else {
            if let Some(url_object) = hash_map.get_mut(&url_string.to_string()) {
                (*url_object).add_reference(host.clone());
            } else {
                panic!("Could not find Url Key");
            }
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

async fn find_all_urls_from_webpage(
    url_to_visit: &String,
    web_client: &mut Client,
    domain_list: Vec<String>,
    hash_map: &mut HashMap<String, Url>,
    not_found_title: &String
) -> Result<Vec<String>, WebScrapingError> {
    web_client.goto(url_to_visit).await?;

    //set response code on url object:
    if let Some(url_object) = hash_map.get_mut(url_to_visit) {
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
            hash_map,
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
            "https://lulzbot.com/3d-printers/".to_string(),
        ];

        let domain = "https://lulzbot.com/".to_string();

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

        let domain = "lulzbot.com/".to_string();

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
            "http://learn.lulzbot.com/learn/".to_string(),
            "/learn/here".to_string(),
        ];

        assert_eq!(
            filter_domains(urls, domains),
            vec![
                "https://lulzbot.com/3d-printers/".to_string(),
                "https://shop.lulzbot.com/3d-printers/".to_string(),
                "http://learn.lulzbot.com/learn/".to_string(),
                "/learn/here".to_string(),
            ]
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
            vec![
                "https://lulzbot.com/3d-printers/".to_string(),
                "https://shop.lulzbot.com/3d-printers/".to_string(),
                "http://learn.lulzbot.com/learn/".to_string(),
                "/learn/here".to_string(),
            ]
        );
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
