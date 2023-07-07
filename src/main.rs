use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::io::Read;
use std::collections::HashMap;

use chrono::prelude::*;
use handlebars::Handlebars;
use serde_json::json;

const VERSION: &str = env!("CARGO_PKG_VERSION");

type Record = HashMap<String, String>;

fn main() {
    println!("Starting the Rust Digger");

    let filepath = "data/data/crates.csv";
    //println!("{}", filepath);
    let result = read_csv_file(filepath);
    println!("Finished reading CSV");
    match result {
        Ok(mut rows) => {
            rows.sort_by(|a, b| b["updated_at"].cmp(&a["updated_at"]));
            generate_pages(rows);
        },
        Err(err) => panic!("Error: {}", err)
    }

    println!("Ending the Rust Digger");
}

fn generate_pages(rows :Vec<Record>) {
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
    //for row in rows {
    //    println!("{:?}", row);
    //    println!("{}", row);
    //}

    //println!("{VERSION}");
    let res = reg.render_template(&template, &json!({
        "version": format!("{VERSION}"),
        "utc": format!("{}", utc),
        "total": rows.len(),
        "rows": &rows[0..100],
    }));
    match res {
        Ok(html) => writeln!(&mut file, "{}", html).unwrap(),
        Err(error) => println!("{}", error)
    }
}


fn read_csv_file(filepath: &str) -> Result<Vec<Record>, Box<dyn Error>> {
    let mut records:Vec<Record> = vec![];
    match File::open(filepath.to_string()) {
        Ok(file) => {
            //let mut content = String::new();
            //file.read_to_string(&mut content).unwrap();
            let mut rdr = csv::Reader::from_reader(file);
            for result in rdr.deserialize() {
                let record: Record = result?;
                records.push(record);
            }
        },
        Err(error) => panic!("Error opening file {}: {}", filepath, error),
    }

    Ok(records)
}
