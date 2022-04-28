use std::fs::File;
use std::path::Path;
use std::fs;
use std::io::Write;

pub enum FormattingError {
    ReadingFileError,
    WritingToFileError
}

pub fn url_index_to_html(json_file: File) -> Result<File, FormattingError> {
    //Read File Contents
    // if let Ok(read_result) = serde_json::from_reader(json_file) {
    //     // Do something with json

    // } else {
    //     return Err(FormattingError::ReadingFileError);
    // }

    ///Check for correct data structure
    /*
    response_code: Option<u16>,
    full_path: String,
    site_references: Vec<String>,
    redirected_to: Option<String>
    */
    //Create HTML document
    let html_file = String::from(
        "<div>Hello World</div>"
    );
    
    write_to_file(html_file, "./data/html.html");
    Ok(json_file)
}

fn write_to_file(
    html_string: String,
    file_path: &str,
) -> Result<(), FormattingError> {
    if let Err(_) = fs::DirBuilder::new().recursive(true).create("./data") {
        println!("Trouble creating data directory!");
        return Err(FormattingError::WritingToFileError);
    }
    if let Ok(mut html_doc) = fs::File::options()
        .write(true)
        .create(true)
        .open(Path::new(file_path))
    {
        if let Ok(_) = html_doc.write(html_string.to_string().as_bytes()) {
            Ok(())
        } else {
            Err(FormattingError::WritingToFileError)
        }
    } else {
        Err(FormattingError::WritingToFileError)
    }
}
