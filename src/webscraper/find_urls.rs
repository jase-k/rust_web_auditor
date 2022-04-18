use fantoccini::elements::Element;
use fantoccini::error::{CmdError, NewSessionError};
use fantoccini::{Client, ClientBuilder, Locator};
use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard};

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

#[derive(Debug)]
pub struct Url {
    url: String,
    response_code: u16,
    site_references: Arc<Mutex<Vec<String>>>,
    redirected_to: Option<String>,
}

impl Url {
    fn new(url: String, response_code: u16, site_reference: String) -> Url {
        let mut new_vec = Vec::new();
        new_vec.push(site_reference);
        Url {
            url: url,
            response_code: response_code,
            site_references: Arc::new(Mutex::new(new_vec)),
            redirected_to: None,
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

    fn add_redirection(&mut self, destination: String) -> &Self {
        self.redirected_to = Some(destination);
        self
    }
    //TODO: add redirection
}

impl PartialEq<Url> for Url {
    fn eq(&self, other: &Url) -> bool {
        if self.url != other.url {
            return false;
        }
        if self.response_code != other.response_code {
            return false;
        }

        let self_vec = self.site_references.lock().unwrap();
        let other_vec = other.site_references.lock().unwrap();
        *self_vec == *other_vec
    }
}

#[derive(Debug)]
pub struct UrlIndex {
    ///urls with a 400-499 response status
    bad_urls: Arc<Mutex<Vec<Url>>>,
    ///urls with a 200-299 response status
    good_urls: Arc<Mutex<Vec<Url>>>,
    ///urls with a 300-399 response status
    redirected_urls: Arc<Mutex<Vec<Url>>>,
    ///urls with a 500+ response status Internal errors.
    error_urls: Arc<Mutex<Vec<Url>>>,
    ///Strings of all urls
    all_urls: Arc<Mutex<HashSet<String>>>,
    ///List of domains that are accepted by the crawler (do not include https / http)
    domain_list: Arc<HashSet<String>>,
}

impl UrlIndex {
    /// Creates a new UrlIndex Object
    /// Must include a Vec of domains that you want to include in the index
    fn new(domains: HashSet<String>) -> UrlIndex {
        //TODO reformat domains to have no "/"
        UrlIndex {
            bad_urls: Arc::new(Mutex::new(vec![])),
            good_urls: Arc::new(Mutex::new(vec![])),
            redirected_urls: Arc::new(Mutex::new(vec![])),
            error_urls: Arc::new(Mutex::new(vec![])),
            all_urls: Arc::new(Mutex::new(HashSet::new())),
            domain_list: Arc::new(domains),
        }
    }

    fn add(&self, url: Url) -> &Self {
        let mut url_vector;
        if url.response_code < 300 {
            url_vector = self.good_urls.lock().unwrap();
        } else if url.response_code < 400 {
            url_vector = self.redirected_urls.lock().unwrap();
        } else if url.response_code < 500 {
            url_vector = self.bad_urls.lock().unwrap();
        } else {
            url_vector = self.error_urls.lock().unwrap();
        }

        (*url_vector).push(url);

        drop(url_vector);
        self
    }

    fn add_to_list(&self, mut urls: Vec<String>, host: String) -> &Self {
        let mut url_vec_guard: MutexGuard<HashSet<String>> = self.all_urls.lock().unwrap();
        urls = self.filter_domains(urls);

        if urls.len() < 1 {
            return self;
        }

        urls = format_urls(host, urls);

        let mut url_iter = urls.iter();

        while let Some(url_string) = url_iter.next() {
            (*url_vec_guard).insert(url_string.to_string());
        }

        self
    }

    fn filter_domains(&self, urls: Vec<String>) -> Vec<String> {
        let domain_iter = self.domain_list.iter();
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
}

pub async fn index_urls() -> Result<UrlIndex, WebScrapingError> {
    let url_index = UrlIndex::new(HashSet::from(["facebook.com".to_string()]));

    // Open web connection
    let mut web_client: Client = open_new_client().await?;

    let host = "https://facebook.com/";

    web_client.goto(&host).await?;

    println!("{}", web_client.current_url().await?);

    let all_urls = find_urls(&mut web_client).await?;

    url_index.add_to_list(all_urls, host.to_string());

    web_client.close().await?;
    // Create up to 10 new pages
    // For each url -> parse urls and add to set.
    // -> parse : (response status, site reference, and url)
    // Err(WebScrapingError::FantocciniCmdErrorr(CmdError))

    Ok(url_index)
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
        }

        if !url.starts_with(&domain) {
            //add domain to url
            (*url).insert_str(0, &domain);
        }
    }
    urls
}

#[cfg(test)]
mod tests {
    use super::*;

    impl PartialEq for UrlIndex {
        fn eq(&self, other: &UrlIndex) -> bool {
            let self_bad_urls = self.bad_urls.lock().unwrap();
            let other_bad_urls = other.bad_urls.lock().unwrap();
            if *self_bad_urls != *other_bad_urls {
                return false;
            }

            let self_good_urls = self.good_urls.lock().unwrap();
            let other_good_urls = other.good_urls.lock().unwrap();
            if *self_good_urls != *other_good_urls {
                return false;
            }

            let self_redirected_urls = self.redirected_urls.lock().unwrap();
            let other_redirected_urls = other.redirected_urls.lock().unwrap();
            if *self_redirected_urls != *other_redirected_urls {
                return false;
            }

            let self_error_urls = self.error_urls.lock().unwrap();
            let other_error_urls = other.error_urls.lock().unwrap();
            if *self_error_urls != *other_error_urls {
                return false;
            }

            let self_all_urls = self.all_urls.lock().unwrap();
            let other_all_urls = other.all_urls.lock().unwrap();
            if *self_all_urls != *other_all_urls {
                return false;
            }

            if *self.domain_list != *other.domain_list {
                return false;
            }

            return true;
        }
    }

    impl Clone for Url {
        fn clone(&self) -> Url {
            let mut new_vec = Vec::new();
            let old_vec = self.site_references.lock().unwrap();
            let mut old_vec_iter = old_vec.iter();

            while let Some(url) = old_vec_iter.next() {
                new_vec.push(url.clone());
            }

            Url {
                url: self.url.clone(),
                response_code: self.response_code.clone(),
                site_references: Arc::new(Mutex::new(new_vec)),
                redirected_to: self.redirected_to.clone(),
            }
        }
    }

    #[test]
    fn url_new_test() {
        let url = Url::new(
            "https://example.com".to_string(),
            200,
            "https://google.com/".to_string(),
        );
        assert_eq!(
            url,
            Url {
                url: "https://example.com".to_string(),
                response_code: 200,
                site_references: Arc::new(Mutex::new(vec!["https://google.com/".to_string()])),
                redirected_to: None
            }
        )
    }

    #[test]
    fn url_add_reference_test() {
        let url = Url::new(
            "https://example.com".to_string(),
            200,
            "https://google.com/".to_string(),
        );
        url.add_reference("https://facebook.com/".to_string());
        assert_eq!(
            url,
            Url {
                url: "https://example.com".to_string(),
                response_code: 200,
                site_references: Arc::new(Mutex::new(vec![
                    "https://google.com/".to_string(),
                    "https://facebook.com/".to_string()
                ])),
                redirected_to: None
            }
        )
    }

    #[test]
    fn url_index_new_test() {
        let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
        assert_eq!(
            url_index,
            UrlIndex {
                bad_urls: Arc::new(Mutex::new(Vec::new())),
                good_urls: Arc::new(Mutex::new(Vec::new())),
                redirected_urls: Arc::new(Mutex::new(Vec::new())),
                error_urls: Arc::new(Mutex::new(Vec::new())),
                all_urls: Arc::new(Mutex::new(HashSet::new())),
                domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
            }
        )
    }

    #[test]
    fn url_index_add_good_url_test() {
        let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
        let url = Url::new(
            "https://example.com".to_string(),
            200,
            "https://google.com/".to_string(),
        );
        let url_copy = url.clone();

        url_index.add(url);

        assert_eq!(
            url_index,
            UrlIndex {
                bad_urls: Arc::new(Mutex::new(Vec::new())),
                good_urls: Arc::new(Mutex::new(vec![url_copy])),
                redirected_urls: Arc::new(Mutex::new(Vec::new())),
                error_urls: Arc::new(Mutex::new(Vec::new())),
                all_urls: Arc::new(Mutex::new(HashSet::new())),
                domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
            }
        )
    }

    #[test]
    fn url_index_add_bad_url_test() {
        let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
        let url = Url::new(
            "https://example.com".to_string(),
            404,
            "https://google.com/".to_string(),
        );
        let url_copy = url.clone();

        url_index.add(url);

        assert_eq!(
            url_index,
            UrlIndex {
                bad_urls: Arc::new(Mutex::new(vec![url_copy])),
                good_urls: Arc::new(Mutex::new(Vec::new())),
                redirected_urls: Arc::new(Mutex::new(Vec::new())),
                error_urls: Arc::new(Mutex::new(Vec::new())),
                all_urls: Arc::new(Mutex::new(HashSet::new())),
                domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
            }
        )
    }

    #[test]
    fn url_index_add_redirected_url_test() {
        let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
        let url = Url::new(
            "https://example.com".to_string(),
            301,
            "https://google.com/".to_string(),
        );
        let url_copy = url.clone();

        url_index.add(url);

        assert_eq!(
            url_index,
            UrlIndex {
                bad_urls: Arc::new(Mutex::new(Vec::new())),
                good_urls: Arc::new(Mutex::new(Vec::new())),
                redirected_urls: Arc::new(Mutex::new(vec![url_copy])),
                error_urls: Arc::new(Mutex::new(Vec::new())),
                all_urls: Arc::new(Mutex::new(HashSet::new())),
                domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
            }
        )
    }

    #[test]
    fn url_index_add_error_url_test() {
        let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
        let url = Url::new(
            "https://example.com".to_string(),
            500,
            "https://google.com/".to_string(),
        );
        let url_copy = url.clone();

        url_index.add(url);

        assert_eq!(
            url_index,
            UrlIndex {
                bad_urls: Arc::new(Mutex::new(Vec::new())),
                good_urls: Arc::new(Mutex::new(Vec::new())),
                redirected_urls: Arc::new(Mutex::new(Vec::new())),
                error_urls: Arc::new(Mutex::new(vec![url_copy])),
                all_urls: Arc::new(Mutex::new(HashSet::from([]))),
                domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
            }
        )
    }

    #[test]
    fn url_index_add_one_all_url_test() {
        let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
        let url = vec!["https://example.com".to_string()];

        url_index.add_to_list(url, "https://example.com".to_string());

        assert_eq!(
            url_index,
            UrlIndex {
                bad_urls: Arc::new(Mutex::new(Vec::new())),
                good_urls: Arc::new(Mutex::new(Vec::new())),
                redirected_urls: Arc::new(Mutex::new(Vec::new())),
                error_urls: Arc::new(Mutex::new(Vec::new())),
                all_urls: Arc::new(Mutex::new(HashSet::from([
                    "https://example.com".to_string()
                ]))),
                domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
            }
        )
    }

    #[test]
    fn url_index_add_multiple_all_url_test() {
        let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
        let url = vec![
            "https://example.com".to_string(),
            "https://example.com/123".to_string(),
            "https://example.com/abc".to_string(),
        ];

        url_index.add_to_list(url, "https://example.com".to_string());

        assert_eq!(
            url_index,
            UrlIndex {
                bad_urls: Arc::new(Mutex::new(Vec::new())),
                good_urls: Arc::new(Mutex::new(Vec::new())),
                redirected_urls: Arc::new(Mutex::new(Vec::new())),
                error_urls: Arc::new(Mutex::new(Vec::new())),
                all_urls: Arc::new(Mutex::new(HashSet::from([
                    "https://example.com".to_string(),
                    "https://example.com/123".to_string(),
                    "https://example.com/abc".to_string()
                ]))),
                domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
            }
        )
    }

    #[test]
    fn url_index_add_test_avoid_duplicates() {
        let url_index = UrlIndex::new(HashSet::from(["https://example.com".to_string()]));
        let url = vec![
            "https://example.com".to_string(),
            "https://example.com/123".to_string(),
            "https://example.com/abc".to_string(),
            "https://example.com/123".to_string(),
            "https://example.com/abc".to_string(),
        ];

        url_index.add_to_list(url, "https://example.com".to_string());

        assert_eq!(
            url_index,
            UrlIndex {
                bad_urls: Arc::new(Mutex::new(Vec::new())),
                good_urls: Arc::new(Mutex::new(Vec::new())),
                redirected_urls: Arc::new(Mutex::new(Vec::new())),
                error_urls: Arc::new(Mutex::new(Vec::new())),
                all_urls: Arc::new(Mutex::new(HashSet::from([
                    "https://example.com".to_string(),
                    "https://example.com/123".to_string(),
                    "https://example.com/abc".to_string(),
                ]))),
                domain_list: Arc::new(HashSet::from(["https://example.com".to_string()]))
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
    fn filter_domains_test() {
        let domains = HashSet::from([
            "lulzbot.com".to_string(),
            "www.lulzbot.com".to_string(),
            "shop.lulzbot.com".to_string(),
            "learn.lulzbot.com".to_string(),
        ]);
        let url_index = UrlIndex::new(domains);

        let urls: Vec<String> = vec![
            "https://lulzbot.com/3d-printers/".to_string(),
            "https://makerbot.com/3d-printers/".to_string(),
            "https://shop.lulzbot.com/3d-printers/".to_string(),
            "http://learn.lulzbot.com/learn/".to_string(),
            "/learn/here".to_string(),
        ];

        assert_eq!(
            url_index.filter_domains(urls),
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
        let domains = HashSet::from([
            "lulzbot.com".to_string(),
            "www.lulzbot.com".to_string(),
            "shop.lulzbot.com".to_string(),
            "learn.lulzbot.com".to_string(),
        ]);
        let url_index = UrlIndex::new(HashSet::from(domains));

        let urls: Vec<String> = vec![
            "https://lulzbot.com/3d-printers/".to_string(),
            "https://makerbot.com/3d-printers/".to_string(),
            "https://shop.lulzbot.com/3d-printers/".to_string(),
            "http://learn.lulzbot.com/learn/".to_string(),
            "http://forum.lulzbot.com/learn/".to_string(),
            "/learn/here".to_string(),
        ];

        assert_eq!(
            url_index.filter_domains(urls),
            vec![
                "https://lulzbot.com/3d-printers/".to_string(),
                "https://shop.lulzbot.com/3d-printers/".to_string(),
                "http://learn.lulzbot.com/learn/".to_string(),
                "/learn/here".to_string(),
            ]
        );
    }

    #[test]
    fn url_add_redirection_test() {
        let mut url = Url::new("https://example.com/base".to_string(), 301, "https://example.com".to_string());
        let destination = String::from("https://example.com/redirected");

        url.add_redirection(destination);

        assert_eq!(url, Url {
            url: String::from("https://example.com/base"),
            response_code: 301, 
            site_references: Arc::new(Mutex::new(vec!["https://example.com".to_string()])),
            redirected_to: Some(String::from("https://example.com/redirected"))
        })
    }
}
