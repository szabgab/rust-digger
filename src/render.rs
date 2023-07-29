use chrono::prelude::{DateTime, Utc};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io::Write;

use crate::{Partials, VERSION};

pub fn read_file(filename: &str) -> String {
    let mut content = String::new();
    match File::open(filename) {
        Ok(mut file) => {
            file.read_to_string(&mut content).unwrap();
        }
        Err(error) => {
            println!("Error opening file {}: {}", filename, error);
        }
    }
    content
}

pub fn load_templates() -> Result<Partials, Box<dyn Error>> {
    // log::info!("load_templates");

    let mut partials = Partials::empty();
    let filename = "templates/incl/header.html";
    partials.add(filename, read_file(filename));
    let filename = "templates/incl/footer.html";
    partials.add(filename, read_file(filename));
    let filename = "templates/incl/list_crates.html";
    partials.add(filename, read_file(filename));

    Ok(partials)
}

pub fn render_static_pages() -> Result<(), Box<dyn Error>> {
    log::info!("render_static_pages");

    let pages = vec![
        ("about", "About Rust Digger"),
        ("support", "Support Rust Digger"),
    ];

    for page in pages {
        let partials = match load_templates() {
            Ok(partials) => partials,
            Err(error) => panic!("Error loading templates {}", error),
        };

        let utc: DateTime<Utc> = Utc::now();
        let globals = liquid::object!({
            "version": format!("{VERSION}"),
            "utc":     format!("{}", utc),
            "title":   page.1,
        });

        let template = liquid::ParserBuilder::with_stdlib()
            .partials(partials)
            .build()
            .unwrap()
            .parse_file(format!("templates/{}.html", page.0))
            .unwrap();
        let html = template.render(&globals).unwrap();

        let mut file = File::create(format!("_site/{}.html", page.0)).unwrap();
        writeln!(&mut file, "{}", html).unwrap();
    }
    Ok(())
}
