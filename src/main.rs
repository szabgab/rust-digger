use std::fs;
use std::fs::File;
use std::io::Write;
use std::io::Read;

use chrono::prelude::*;
use handlebars::Handlebars;
use serde_json::json;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    println!("Starting the Rust Digger");
    generate_pages();
    println!("Ending the Rust Digger");
}

fn generate_pages() {
    let reg = Handlebars::new();

    let utc: DateTime<Utc> = Utc::now();

    let template_file = "templates/index.html";
    let mut template = String::new();
    match File::open(template_file) {
        Ok(mut file) => {
            file.read_to_string(&mut template).unwrap();
        },
        Err(error) => {
            println!("Error opening file {}: {}", template_file, error);
        },
    }

    // Create a folder _site
    let _res = fs::create_dir_all("_site");

    // Create an html page _site/index.html with the title
    let filename = "_site/index.html";
    let mut file = File::create(filename).unwrap();

    //println!("{VERSION}");
    let res = reg.render_template(&template, &json!({"version": format!("{VERSION}"), "utc": format!("{}", utc)}));
    match res {
        Ok(html) => writeln!(&mut file, "{:?}", html).unwrap(),
        Err(error) => println!("{}", error)
    }
}


