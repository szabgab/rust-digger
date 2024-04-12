use std::error::Error;
use std::fs;

use clap::Parser;
use flate2::read::GzDecoder;
use reqwest::header::USER_AGENT;
use tar::Archive;
use tempdir::TempDir;

mod macros;
use macros::ok_or_exit;

use rust_digger::{crates_root, read_crates, read_versions, Crate, CrateVersion};

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

    let args = Cli::parse();

    if args.limit > 0 {
        log::info!("Download {} crates.", args.limit);
    } else {
        log::info!("We currently don't support downloading all the crates. Use --limit 10");
        return;
    }

    fs::create_dir_all(crates_root()).unwrap();
    // load list of crates with version numbers
    let crates: Vec<Crate> = ok_or_exit!(read_crates(0), 2);
    let versions: Vec<CrateVersion> = ok_or_exit!(read_versions(), 2);

    match download_crates(&crates, &versions, args.limit) {
        Ok(()) => {}
        Err(err) => log::error!("Error: {err}"),
    }

    // remove old versions of the same crates??

    log::info!("Elapsed time: {} sec.", start_time.elapsed().as_secs());
    log::info!("End downloading crates");
}

fn download_crates(
    crates: &[Crate],
    versions: &[CrateVersion],
    limit: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("start update repositories");

    let mut count = 0;
    for krate in crates {
        if 0 < limit && limit <= count {
            break;
        }

        //log::info!("update_at {}", krate.updated_at); // 2023-09-18 01:44:10.299066
        log::info!(
            "Crate {} updated_at: {}   {}",
            krate.name,
            krate.updated_at,
            krate.id
        );
        // TODO Maybe crate a HashMap for faster lookup, though we are downloading from the internet so local CPU and runtime is probably not an issue
        let mut filtered_versions = versions
            .iter()
            .filter(|version| version.crate_id == krate.id)
            .collect::<Vec<_>>();
        if filtered_versions.is_empty() {
            log::error!("No version of {} could be found", krate.name);
            continue;
        }

        filtered_versions.sort_by_cached_key(|ver| &ver.created_at);
        filtered_versions.reverse();

        // for ver in &filtered_versions[0..core::cmp::min(5, filtered_versions.len())] {
        //     log::info!("Crate {} version: {}", krate.name, ver.num);
        // }

        // if filtered_versions.len() > 1 {
        //     log::error!("More than 1 version of {} were found {}", krate.name, filtered_versions.len());
        //     continue;
        // }

        // TODO maybe we should not include the versions that are not in the standard format e.g. only accept  0.3.0 and not  0.3.0-beta-dev.30 ?
        //log::info!("version of {}: {:#?}", krate.name, filtered_versions[0]);

        let folder = crates_root().join(format!("{}-{}", krate.name, filtered_versions[0].num));
        log::info!("Checking {:?}", folder);
        if folder.exists() {
            log::info!("{:?} already exists. Skipping download", folder);
            continue;
        }

        // "https://crates.io/api/v1/crates/serde/1.0.0/download
        let url = format!(
            "https://crates.io/api/v1/crates/{}/{}/download",
            krate.name, filtered_versions[0].num
        );

        log::info!("url {url}");

        let downloaded_file = download_crate(&url).unwrap();
        extract_file(&downloaded_file).unwrap();
        std::fs::remove_file(downloaded_file).unwrap();

        count += 1;
    }

    Ok(())
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
        return Err(Box::<dyn Error>::from("failed fetching {url}"));
    };

    if response.status() != 200 {
        log::error!("status was {:?} when fetching {}", response.status(), url);
    }

    let download_file = std::path::Path::new("download.tar.gz");
    let mut file = fs::File::create(download_file).unwrap();

    let total = std::io::copy(&mut response, &mut file)
        .expect("should copy fetched response into created file");
    log::info!("Total downloaded: {total}");

    Ok(download_file.to_path_buf())
}

fn extract_file(file: &std::path::PathBuf) -> Result<(), Box<dyn Error>> {
    let tar_gz = fs::File::open(file)?;
    let tar = GzDecoder::new(tar_gz);
    let tmp_dir = TempDir::new("example").unwrap();
    log::info!("tempdir: {:?}", tmp_dir);

    let mut archive = Archive::new(tar);
    archive.unpack(&tmp_dir)?;

    let extracted_dir = fs::read_dir(std::path::Path::new(tmp_dir.path()))?
        .next()
        .unwrap()
        .unwrap();
    log::info!("extra: {:?}", extracted_dir);
    log::info!("extra: {:?}", extracted_dir.file_name());

    fs::rename(
        extracted_dir.path(),
        crates_root().join(extracted_dir.file_name()),
    )?;

    Ok(())
}
