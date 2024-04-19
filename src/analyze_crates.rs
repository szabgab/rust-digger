use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use clap::Parser;

use rust_digger::{analyzed_crates_root, crates_root, create_data_folders};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CrateDetails {
    has_build_rs: bool,
}

impl CrateDetails {
    const fn new() -> Self {
        Self {
            has_build_rs: false,
        }
    }
}

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(
        long,
        default_value_t = 0,
        help = "Limit the number of repos we process."
    )]
    limit: usize,
}

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    log::info!("Start analyzing crates.");
    let start_time = std::time::Instant::now();

    match run() {
        Ok(()) => {}
        Err(err) => log::error!("Error: {err}"),
    }

    log::info!("Elapsed time: {} sec.", start_time.elapsed().as_secs());
    log::info!("End analyzing crates");
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    log::info!("Limit: {}", args.limit);

    collect_data_from_crates(args.limit)?;

    Ok(())
}

fn collect_data_from_crates(limit: usize) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("collect_data_from_crates");
    if 0 < limit {
        log::info!("We are going to process only {} crates", limit);
    } else {
        log::info!("We are going to process all the crates we find locally",);
    }
    create_data_folders()?;

    for (count, entry) in crates_root().read_dir()?.enumerate() {
        if limit > 0 && count > limit {
            break;
        }
        let dir_entry = entry?;
        log::info!("{:?}", dir_entry);

        // try to read the already collected data
        // if it succeeds break
        // if it fails collect all the data and save to the disk
        let mut details = CrateDetails::new();
        has_files(&dir_entry.path(), &mut details)?;
        log::info!("details: {details:#?}");
        if let Some(filename) = dir_entry.path().file_name() {
            save_details(&details, filename)?;
        } else {
            log::error!("Could not get file_name");
        }
    }

    Ok(())
}

fn has_files(path: &PathBuf, details: &mut CrateDetails) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("has_files for {path:?}");

    details.has_build_rs = path.join("build.rs").exists();

    Ok(())
}

fn save_details(
    details: &CrateDetails,
    filename: &OsStr,
) -> Result<(), Box<dyn std::error::Error>> {
    let filepath = analyzed_crates_root().join(filename);
    log::info!("Saving crate details to {filepath:?}");
    let mut file = File::create(filepath)?;
    writeln!(&mut file, "{}", serde_json::to_string(details)?)?;

    Ok(())
}
