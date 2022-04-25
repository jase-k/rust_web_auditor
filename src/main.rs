mod webdriver;
mod webscraper;
use clap::{crate_authors, crate_description, Arg, Command};
use webscraper::find_urls::{index_urls, WebScrapingError};

#[tokio::main]
async fn main() -> Result<(), WebScrapingError> {
    if cfg!(target_os = "windows") {
        println!("Running configuration for windows");
    } else if cfg!(target_os = "linux") {
        println!("Running configuration for linux");
    }

    let matches = Command::new("Web-audit")
        .author(crate_authors!("\n"))
        .version("0.0.0")
        .about(crate_description!())
        .subcommand(
            Command::new("index-urls")
                .arg(
                    Arg::new("starting-url")
                        .long("url")
                        .short('u')
                        .takes_value(true)
                        .help("Provide the entry point for your url aggregation")
                )
                .arg(
                    Arg::new("domain-list")
                        .long("domain-list")
                        .takes_value(true)
                        .help("Provide the list of domains you want audited")
                        .long_help("Provide the file to the csv file you have your domains you want to be audited \n Example: --domain-list 'C:users/name/domains.txt'"),
                )
                .arg(
                    Arg::new("404-title")
                        .long("404")
                        .takes_value(true)
                        .default_value("Page Not Found")
                        .help("The title of your 404 page. <title>Page Not Found</title> = 'Page Not Found'")
                        .long_help("The webscraper checks to see if the page is a 404 by checking the page title element. Make sure this title is unique to your 404 page for best results. If you don't know your 404 page title go to https://your-web-domain.com/lajdfjadsjl and inspect the page. (right click inspect). In the console type 'document.querySelector('title') It will output your title element. The value passed in only needs to contain part of the title"),
                )
        )
        .get_matches();

    if let Some(sub_matches) = matches.subcommand_matches("index-urls") {
        let not_found_title; 
        let url; 
        let domains;

        if let Some(url_) = sub_matches.value_of("starting-url") {
            url = url_
        } else {
            panic!("url must be provided");
        }

        if let Some(domains_) = sub_matches.value_of("domain-list") {
            //TODO: convert file path to vec. 
            domains = vec![domains_.to_string()];
        } else {
            domains = vec![(&url).clone().to_string()]
        }

        if let Some(not_found_title_) = sub_matches.value_of("404-title") {
            not_found_title = not_found_title_;
        } else {
            not_found_title = "Page Not Found";
        }
        
        index_urls(
            url.to_string(),
            domains,
            not_found_title.to_string()
        )
        .await?;
    };
    Ok(())
}
