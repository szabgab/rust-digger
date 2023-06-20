use std::fs;
use std::fs::File;
use std::io::Write;


fn main() {
    println!("Rust Digger");

    // Create a folder _site
    let _res = fs::create_dir_all("_site");

    // Create an html page _site/index.html with the title
    let filename = "_site/index.html";
    let mut file = File::create(filename).unwrap();
    let html = "
        <html>
            <head>
               <title>Rust Digger</title>
            </head>
            <body>
              <h1>Rust Digger</h1>
            </body>
        </html>";
    writeln!(&mut file, "{html}").unwrap();
}
