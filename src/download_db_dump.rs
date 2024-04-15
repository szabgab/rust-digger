use flate2::read::GzDecoder;
use std::{fs, io};
use tar::Archive;

use rust_digger::{create_data_folders, get_db_dump_folder, get_temp_folder};

fn year_of_yesterday() -> String {
    let now = chrono::Local::now();
    let yesterday = now - chrono::Duration::try_days(1).unwrap();
    yesterday.format("%Y").to_string()
}

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    download();
    extract();
}

fn download() {
    log::info!("Start downloading db-dump.tar.gz");
    let start_time = std::time::Instant::now();

    let db_archive = get_temp_folder().join("db-dump.tar.gz");
    create_data_folders().unwrap();

    if fs::metadata(&db_archive).is_ok() {
        fs::remove_file(&db_archive).expect("should remove previous database archive");
    }

    let mut response = reqwest::blocking::get("https://static.crates.io/db-dump.tar.gz")
        .expect("should fetch new database archive from static.crates.io");

    log::info!("db_archive: {:?}", &db_archive);
    let mut file =
        fs::File::create(&db_archive).expect("should create new file to write database archive to");
    let total =
        io::copy(&mut response, &mut file).expect("should copy fetched response into created file");
    log::info!("Total downloaded: {total}");
    log::info!(
        "Elapsed time for download: {} sec.",
        start_time.elapsed().as_secs()
    );
}

fn extract() {
    log::info!("Start extracting db-dump.tar.gz");
    let start_time = std::time::Instant::now();

    let data_dir = get_db_dump_folder();

    if fs::metadata(&data_dir).is_ok() {
        fs::remove_dir_all(&data_dir).expect("should remove previously extracted data");
    }

    let db_archive = get_temp_folder().join("db-dump.tar.gz");
    let tar_gz = fs::File::open(db_archive).expect("should open new database archive file");
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive
        .unpack(get_temp_folder())
        .expect("should unpack archive file into current directory");

    let extracted_dir = fs::read_dir(get_temp_folder())
        .expect("should read directory extracted from archive")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_str().unwrap().to_owned())
        .filter(|entry| entry.starts_with(&year_of_yesterday()))
        .map(|entry| entry.split('/').next().unwrap().to_owned())
        .next()
        .expect("should find name of directory extracted from archive");

    let extracted_folder = get_temp_folder().join(extracted_dir);

    log::info!("rename {extracted_folder:?} to {data_dir:?}");
    fs::rename(extracted_folder, data_dir).expect("should rename extracted directory to 'data'");

    log::info!(
        "Elapsed time for extraction: {} sec.",
        start_time.elapsed().as_secs()
    );
    log::info!("Extraction process ended");
}
