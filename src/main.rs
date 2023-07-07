use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
//use std::io::Read;
use std::collections::HashMap;

use chrono::prelude::*;
use handlebars::Handlebars;
use serde_json::json;

const VERSION: &str = env!("CARGO_PKG_VERSION");

type Record = HashMap<String, String>;

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    log::info!("Starting the Rust Digger");

    let args: Vec<String> = env::args().collect();
    let limit;
    if args.len() == 2 {
        limit = args[1].parse().expect("Could not convert to i32");
    } else {
        limit = 0;
    }
    log::info!("Limit {limit}");

    let filepath = "data/data/crates.csv";
    //log::info!("{}", filepath);
    let result = read_csv_file(filepath, limit);
    log::info!("Finished reading CSV");
    match result {
        Ok(mut rows) => {
            rows.sort_by(|a, b| b["updated_at"].cmp(&a["updated_at"]));
            match generate_pages(rows) {
                Ok(_) => {},
                Err(err) => panic!("Error: {}", err)
            }
        },
        Err(err) => panic!("Error: {}", err)
    }

    log::info!("Ending the Rust Digger");
}

fn generate_pages(rows :Vec<Record>) -> Result<(), Box<dyn Error>> {
    let mut reg = Handlebars::new();
    reg.register_template_file("about", "templates/about.html")?;
    reg.register_template_file("index", "templates/index.html")?;
    reg.register_template_file("layout", "templates/layout.html")?;
    let utc: DateTime<Utc> = Utc::now();

    // Create a folder _site
    let _res = fs::create_dir_all("_site");

    // Create an html page _site/index.html with the title
    let filename = "_site/index.html";
    let mut file = File::create(filename).unwrap();
    //for row in rows {
    //    println!("{:?}", row);
    //    println!("{}", row);
    //}

    //log::info!("{VERSION}");
    const PAGE_SIZE: usize = 100;
    let page_size = if rows.len() > PAGE_SIZE { PAGE_SIZE } else { rows.len() };
    let res = reg.render("index", &json!({
        "version": format!("{VERSION}"),
        "utc": format!("{}", utc),
        "total": rows.len(),
        "rows": &rows[0..page_size],
        "title": "Rust Digger",
        "parent": "layout",
    }));
    match res {
        Ok(html) => writeln!(&mut file, "{}", html).unwrap(),
        Err(error) => println!("{}", error)
    }

    let filename = "_site/about.html";
    let mut file = File::create(filename).unwrap();

    let res = reg.render("about", &json!({
        "version": format!("{VERSION}"),
        "utc": format!("{}", utc),
        "title": "About Rust Digger",
        "parent": "layout",
    }));
    match res {
        Ok(html) => writeln!(&mut file, "{}", html).unwrap(),
        Err(error) => println!("{}", error)
    }


    Ok(())
}


fn read_csv_file(filepath: &str, limit: i32) -> Result<Vec<Record>, Box<dyn Error>> {
    let mut records:Vec<Record> = vec![];
    let mut count = 0;
    match File::open(filepath.to_string()) {
        Ok(file) => {
            //let mut content = String::new();
            //file.read_to_string(&mut content).unwrap();
            let mut rdr = csv::Reader::from_reader(file);
            for result in rdr.deserialize() {
                count += 1;
                if limit > 0 && count >= limit {
                    println!("Limit of {limit} reached");
                    break
                }
                let record: Record = result?;
                records.push(record);
            }
        },
        Err(error) => panic!("Error opening file {}: {}", filepath, error),
    }

    Ok(records)
}
