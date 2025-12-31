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
use thousands::Separable as _;

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
    log::info!(
        "Found {} crates and {} versions",
        crates.len().separate_with_commas(),
        versions.len().separate_with_commas()
    );

    let (newest_crates, downloaded_count, downloaded_total) =
        download_crates(&crates, &versions, args.limit)?;

    // If the limit is not 0 we don't have all the crates in the newest_crates HashSet so we should not remove the old versions based on that.
    // TODO: have a set that contains all the newest crates and then remove the old versions based on that.
    if args.limit == 0 {
        remove_old_versions_of_the_crates(&newest_crates)?;
    }

    let crate_folders = crates_root().read_dir()?.flatten().count();

    log::info!(
        "Total number of downloaded crates: {}  Total crates: {}",
        crate_folders.separate_with_commas(),
        crates.len().separate_with_commas()
    );

    log::info!(
        "Total downloaded size: {} bytes in {} crates",
        downloaded_total.separate_with_commas(),
        downloaded_count.separate_with_commas()
    );
    Ok(())
}

/// Go over the downloaded crates on disk.
/// Check each one of them of it is in the `HashSet` of most recent crates.
/// Remove the ones that are not there.
fn remove_old_versions_of_the_crates(
    newest_versions: &HashSet<OsString>,
) -> Result<(), Box<dyn Error>> {
    log::info!("start remove_old_versions_of_the_crates");

    for entry in crates_root().read_dir()?.flatten() {
        log::info!("entry: {:?}", entry.file_name().display());

        if !newest_versions.contains(&entry.file_name()) {
            log::info!("removing old crate: {:?}", entry.path().display());

            match std::fs::remove_dir_all(entry.path()) {
                Ok(()) => log::info!("file {:?} removed", entry.path().display()),
                Err(err) => log::error!("{err}"),
            }
        }
    }

    log::info!("end remove_old_versions_of_the_crates");
    Ok(())
}

/// Download the crates from crates.io and extract them to the `crates_root` folder.
/// Returns a tuple with the set of newest versions and the total size downloaded in bytes.
fn download_crates(
    crates: &[Crate],
    versions: &[CrateVersion],
    limit: u32,
) -> Result<(HashSet<OsString>, u32, u64), Box<dyn Error>> {
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
        }
    }

    let mut count = 0;
    let mut total = 0;
    for krate in crates {
        if 0 < limit && limit <= count {
            break;
        }

        log::info!("----------");
        log::info!(
            "Crate: {} updated_at: {}  id: {}",
            krate.name,
            krate.updated_at,
            krate.id
        );

        let krate_name_version = format!("{}-{}", krate.name, latest[&krate.id].num);
        newest_versions.insert(OsString::from(&krate_name_version));

        let folder = crates_root().join(krate_name_version);
        log::info!("Checking {:?}", folder.display());

        if folder.exists() {
            log::info!("{:?} already exists. Skipping download", folder.display());
            continue;
        }

        // "https://crates.io/api/v1/crates/serde/1.0.0/download
        let url = format!(
            "https://crates.io/api/v1/crates/{}/{}/download",
            krate.name, latest[&krate.id].num
        );

        log::info!("downloading url {url}");

        match download_crate(&url) {
            Ok((downloaded_file, size)) => {
                count += 1;
                total += size;
                log::info!(
                    "Downloaded: {} (so far download {} crates with a total of {} bytes)",
                    size.separate_with_commas(),
                    count.separate_with_commas(),
                    total.separate_with_commas()
                );
                match extract_file(&downloaded_file) {
                    Ok(filename) => log::info!("extracted {:?}", filename.display()),
                    Err(err) => log::error!("{err} {url}"),
                }

                match std::fs::remove_file(&downloaded_file) {
                    Ok(()) => log::info!("file {:?} removed", downloaded_file.display()),
                    Err(err) => log::error!("{err}"),
                }
            }
            Err(err) => log::error!("{err}"),
        }
    }

    Ok((newest_versions, count, total))
}

fn download_crate(url: &str) -> Result<(std::path::PathBuf, u64), Box<dyn Error>> {
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
    let mut file = fs::File::create(&download_file)?;

    let total = std::io::copy(&mut response, &mut file)
        .map_err(|err| format!("Failed to copy response into file: {err}"))?;

    Ok((download_file, total))
}

fn extract_file(file: &std::path::PathBuf) -> Result<OsString, Box<dyn Error>> {
    let tar_gz = fs::File::open(file)?;
    let tar = GzDecoder::new(tar_gz);
    let tmp_dir = TempDir::new_in(get_temp_folder(), "example")?;
    log::info!("tempdir: {tmp_dir:?}");

    let mut archive = Archive::new(tar);
    archive.unpack(&tmp_dir)?;

    let extracted_dir = fs::read_dir(std::path::Path::new(tmp_dir.path()))?
        .next()
        .ok_or("Could not extract file")??;
    log::info!("extract dir: {extracted_dir:?}");
    log::info!("extract filename {:?}", extracted_dir.file_name().display());

    fs::rename(
        extracted_dir.path(),
        crates_root().join(extracted_dir.file_name()),
    )?;

    Ok(extracted_dir.file_name())
}
