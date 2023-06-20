use std::fs;
use std::fs::File;
use std::io::Write;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    println!("Rust Digger");

    generate_pages();
}

fn generate_pages() {
    // Create a folder _site
    let _res = fs::create_dir_all("_site");

    // Create an html page _site/index.html with the title
    let filename = "_site/index.html";
    let mut file = File::create(filename).unwrap();
    let html = format!("
        <html>
            <head>
               <title>Rust Digger</title>
            </head>
            <body>
              <h1>Rust Digger</h1>
              Version {VERSION}
              <a href=\"https://github.com/szabgab/rust-digger\">source</a>
            </body>
        </html>");

    writeln!(&mut file, "{html}").unwrap();
}
