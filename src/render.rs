use chrono::prelude::{DateTime, Utc};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use crate::{Crate, Partials, PAGE_SIZE, VERSION};

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

pub fn render_list_page(
    filename: &String,
    title: &String,
    crates: &Vec<&Crate>,
) -> Result<(), Box<dyn Error>> {
    // log::info!("render {filename}");

    let partials = match load_templates() {
        Ok(partials) => partials,
        Err(error) => panic!("Error loading templates {}", error),
    };

    let page_size = if crates.len() > PAGE_SIZE {
        PAGE_SIZE
    } else {
        crates.len()
    };

    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   title,
        "total":   crates.len(),
        "crates":  (&crates[0..page_size]).to_vec(),
    });

    let template = liquid::ParserBuilder::with_stdlib()
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/crate_list_page.html")
        .unwrap();
    let html = template.render(&globals).unwrap();

    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{}", html).unwrap();
    //match res {
    //    Ok(html) => writeln!(&mut file, "{}", html).unwrap(),
    //    Err(error) => println!("{}", error)
    //}
    Ok(())
}

pub fn render_news_pages() {
    log::info!("render_news_pages");
    let utc: DateTime<Utc> = Utc::now();

    let path = Path::new("templates/news");
    for entry in path.read_dir().expect("read_dir call failed") {
        if let Ok(entry) = entry {
            let partials = match load_templates() {
                Ok(partials) => partials,
                Err(error) => panic!("Error loading templates {}", error),
            };
            if entry.path().extension().unwrap() != "html" {
                continue;
            }

            log::info!("news file: {:?}", entry.path());
            log::info!("{:?}", entry.path().strip_prefix("templates/"));
            let output_path = Path::new("_site")
                .join(entry.path().strip_prefix("templates/").unwrap().as_os_str());
            let template = liquid::ParserBuilder::with_stdlib()
                .partials(partials)
                .build()
                .unwrap()
                .parse_file(entry.path())
                .unwrap();

            let globals = liquid::object!({
                "version": format!("{VERSION}"),
                "utc":     format!("{}", utc),
            });
            let html = template.render(&globals).unwrap();
            //let filename = "_site/news.html";
            let mut file = File::create(output_path).unwrap();
            writeln!(&mut file, "{}", html).unwrap();
        }
    }

    //            },
    //            Err(error) => {
    //                println!("Error opening file {:?}: {}", file.as_os_str(), error);
    //            },
    //        }
    //    }
}
