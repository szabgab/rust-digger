use core::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::ffi::OsString;
use std::fs;

use clap::Parser;
use flate2::read::GzDecoder;
use reqwest::header::USER_AGENT;
use tar::Archive;
use tempdir::TempDir;

use rust_digger::{
    crates_root, create_data_folders, get_temp_folder, read_crates, read_versions, Crate,
    CrateVersion,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(
        long,
        default_value_t = 0,
        help = "Limit the number of crates to download."
    )]
    limit: u32,
}

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    log::info!("Start downloading crates");
    let start_time = std::time::Instant::now();

    match run() {
        Ok(()) => {}
        Err(err) => log::error!("Error: {err}"),
    }

    log::info!("Elapsed time: {} sec.", start_time.elapsed().as_secs());
    log::info!("End downloading crates");
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    if args.limit > 0 {
        log::info!("Download {} crates.", args.limit);
    } else {
        log::info!("Downloading all the crates.");
    }

    create_data_folders()?;

    let crates: Vec<Crate> = read_crates(0)?;
    let versions: Vec<CrateVersion> = read_versions()?;

    let newest_crates = download_crates(&crates, &versions, args.limit)?;

    remove_old_versions_of_the_crates(&newest_crates)?;

    Ok(())
}

fn remove_old_versions_of_the_crates(
    newest_versions: &HashSet<OsString>,
) -> Result<(), Box<dyn Error>> {
    log::info!("start remove_old_versions_of_the_crates");

    for entry in crates_root().read_dir()?.flatten() {
        //log::info!("entry: {:?}", entry.file_name());

        if !newest_versions.contains(&entry.file_name()) {
            log::info!("removing old crate: {:?}", entry.path());

            match std::fs::remove_dir_all(entry.path()) {
                Ok(()) => log::info!("file {:?} removed", entry.path()),
                Err(err) => log::error!("{err}"),
            };
        }
    }

    // check each one of them in the list of most recent crates
    // remove the ones that are not in the most recent list
    log::info!("end remove_old_versions_of_the_crates");
    Ok(())
}

fn download_crates(
    crates: &[Crate],
    versions: &[CrateVersion],
    limit: u32,
) -> Result<HashSet<OsString>, Box<dyn Error>> {
    log::info!("start update repositories");

    let mut newest_versions: HashSet<OsString> = HashSet::new();

    // TODO maybe we should not include the versions that are not in the standard format e.g. only accept  0.3.0 and not  0.3.0-beta-dev.30 ?
    let mut latest: HashMap<String, CrateVersion> = HashMap::new();
    for version in versions {
        match latest.get(&version.crate_id) {
            Some(current_version) => {
                if current_version.created_at.cmp(&version.created_at) == Ordering::Less {
                    latest.insert(version.crate_id.clone(), version.clone());
                }
            }
            None => {
                latest.insert(version.crate_id.clone(), version.clone());
            }
        };
    }

    let mut count = 0;
    for krate in crates {
        if 0 < limit && limit <= count {
            break;
        }

        log::info!(
            "Crate: {} updated_at: {}  id: {}",
            krate.name,
            krate.updated_at,
            krate.id
        );

        let krate_name_version = format!("{}-{}", krate.name, latest[&krate.id].num);
        newest_versions.insert(OsString::from(&krate_name_version));

        let folder = crates_root().join(krate_name_version);
        log::info!("Checking {:?}", folder);

        if folder.exists() {
            log::info!("{:?} already exists. Skipping download", folder);
            continue;
        }

        // "https://crates.io/api/v1/crates/serde/1.0.0/download
        let url = format!(
            "https://crates.io/api/v1/crates/{}/{}/download",
            krate.name, latest[&krate.id].num
        );

        log::info!("downloading url {url}");

        match download_crate(&url) {
            Ok(downloaded_file) => {
                match extract_file(&downloaded_file) {
                    Ok(filename) => log::info!("extracted {filename:?}"),
                    Err(err) => log::error!("{err}"),
                };

                match std::fs::remove_file(&downloaded_file) {
                    Ok(()) => log::info!("file {downloaded_file:?} removed"),
                    Err(err) => log::error!("{err}"),
                };
            }
            Err(err) => log::error!("{err}"),
        }

        count += 1;
    }

    Ok(newest_versions)
}

fn download_crate(url: &str) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let client = reqwest::blocking::Client::new();

    let Ok(mut response) = client
        .get(url)
        .header(
            USER_AGENT,
            format!("Rust Digger {VERSION} https://rust-digger.code-maven.com/"),
        )
        .send()
    else {
        return Err(Box::<dyn Error>::from(format!("failed fetching {url}")));
    };

    if response.status() != 200 {
        return Err(Box::<dyn Error>::from(format!(
            "status was {:?} when fetching {}",
            response.status(),
            url
        )));
    }

    let download_file = get_temp_folder().join("download.tar.gz");
    let mut file = fs::File::create(&download_file).unwrap();

    let total = std::io::copy(&mut response, &mut file)
        .expect("should copy fetched response into created file");
    log::info!("Total downloaded: {total}");

    Ok(download_file)
}

fn extract_file(file: &std::path::PathBuf) -> Result<OsString, Box<dyn Error>> {
    let tar_gz = fs::File::open(file)?;
    let tar = GzDecoder::new(tar_gz);
    let tmp_dir = TempDir::new_in(get_temp_folder(), "example")?;
    log::info!("tempdir: {:?}", tmp_dir);

    let mut archive = Archive::new(tar);
    archive.unpack(&tmp_dir)?;

    let extracted_dir = fs::read_dir(std::path::Path::new(tmp_dir.path()))?
        .next()
        .ok_or("Could not extract file")??;
    log::info!("extract dir: {:?}", extracted_dir);
    log::info!("extract filename {:?}", extracted_dir.file_name());

    fs::rename(
        extracted_dir.path(),
        crates_root().join(extracted_dir.file_name()),
    )?;

    Ok(extracted_dir.file_name())
}
