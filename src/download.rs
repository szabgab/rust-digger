use flate2::read::GzDecoder;
use std::{fs, io, path};
use tar::Archive;

fn year_of_yesterday() -> String {
    let now = chrono::Local::now();
    let yesterday = now - chrono::Duration::try_days(1).unwrap();
    yesterday.format("%Y").to_string()
}

fn main() {
    let data_dir = "./data";
    let db_archive = "./db-dump.tar.gz";

    if fs::metadata(data_dir).is_ok() {
        fs::remove_dir_all(data_dir).expect("should remove previously extracted data");
    }
    if fs::metadata(path::Path::new(db_archive)).is_ok() {
        fs::remove_file(db_archive).expect("should remove previous database archive");
    }

    let mut response = reqwest::blocking::get("https://static.crates.io/db-dump.tar.gz")
        .expect("should fetch new database archive from static.crates.io");

    let mut file = fs::File::create(path::Path::new(db_archive))
        .expect("should create new file to write database archive to");
    let _ =
        io::copy(&mut response, &mut file).expect("should copy fetched response into created file");

    let tar_gz = fs::File::open(db_archive).expect("should open new database archive file");
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive
        .unpack(".")
        .expect("should unpack archive file into current directory");

    let extracted_dir = fs::read_dir(path::Path::new("."))
        .expect("should read directory extracted from archive")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_str().unwrap().to_owned())
        .filter(|entry| entry.starts_with(&year_of_yesterday()))
        .map(|entry| entry.split('/').next().unwrap().to_owned())
        .next()
        .expect("should find name of directory extracted from archive");

    fs::rename(extracted_dir, "data").expect("should rename extracted directory to 'data'");
}
