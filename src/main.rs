mod formatter;
mod webdriver;
mod webscraper;
use clap::{crate_authors, crate_description, Arg, Command};
use std::fs;
use std::path::Path;
use webscraper::find_urls::{index_urls, WebScrapingError};
use formatter::formatter::url_index_to_html;

#[tokio::main]
async fn main() -> Result<(), WebScrapingError> {
    if cfg!(target_os = "windows") {
        println!("Running configuration for windows");
    } else if cfg!(target_os = "linux") {
        println!("Running configuration for linux");
    }
    let matches = Command::new("web-audit")
    .author(crate_authors!("\n"))
    .version("0.0.0")
    .about(crate_description!())
    .subcommand(
        //TODO : Allow domain list from file
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
    .subcommand(
        Command::new("convert_json_html")
            .arg(
                Arg::new("file-path")
                .long("file-path")
                .short('f')
                .takes_value(true)
                .help("Path to the file of the json you want to convert to html")
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

        if let Some(domains_path) = sub_matches.value_of("domain-list") {
            //TODO: convert file path to vec.
            let file_content = fs::read_to_string(Path::new(domains_path));
            if let Ok(domain_string) = file_content {
                let domains_: Vec<String> =
                    domain_string.split(",").map(|x| x.to_string()).collect();
                domains = domains_
            } else {
                panic!("Could not read domain file path!");
            }
        } else {
            domains = vec![(&url).clone().to_string()]
        }

        if let Some(not_found_title_) = sub_matches.value_of("404-title") {
            not_found_title = not_found_title_;
        } else {
            not_found_title = "Page Not Found";
        }

        index_urls(url.to_string(), domains, not_found_title.to_string()).await?;
    };

    if let Some(sub_matches) = matches.subcommand_matches("convert_json_html"){
        let file;
        if let Some(file_path) = sub_matches.value_of("file-path") {
            if let Ok(file_) = fs::File::options().read(true).open(file_path) {
                file = file_;
            } else {
                panic!("cannot read file!")
            }
        } else {
            panic!("file-path must be provided");
        }

        url_index_to_html(file);
    }
    Ok(())
}
