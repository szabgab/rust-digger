use std::fs::File;
use std::io::Read;

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
