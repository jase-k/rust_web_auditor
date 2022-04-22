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
        )
        .get_matches();

    if let Some(sub_matches) = matches.subcommand_matches("index-urls") {
        if let Some(url) = sub_matches.value_of("starting-url") {
            index_urls(
                url.to_string(),
                vec![url.to_string()],
            )
            .await?;
        }
    };


    Ok(())
}
