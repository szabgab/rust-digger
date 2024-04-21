use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use clap::Parser;
use walkdir::WalkDir;

use rust_digger::{analyzed_crates_root, crates_root, create_data_folders, CrateDetails};

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
        if limit > 0 && count >= limit {
            break;
        }
        let dir_entry = entry?;
        log::info!("{:?}", dir_entry);

        let filepath = if let Some(filename) = dir_entry.path().file_name() {
            analyzed_crates_root().join(filename)
        } else {
            log::error!("Could not get file_name");
            continue;
        };

        // try to read the already collected data, if it succeeds go to the next crate
        if let Ok(content) = std::fs::read_to_string(&filepath) {
            if let Ok(_details) = serde_json::from_str::<CrateDetails>(&content) {
                log::info!("Details found");
                continue;
            }
        }

        // if it fails collect all the data and save to the disk
        let mut details = CrateDetails::new();
        has_files(&dir_entry.path(), &mut details)?;
        log::info!("details: {details:#?}");

        details.size = disk_size(&dir_entry.path());

        save_details(&details, filepath)?;
    }

    Ok(())
}

fn disk_size(root: &PathBuf) -> u64 {
    let mut size = 0;
    for dir_entry in WalkDir::new(root).into_iter().flatten() {
        if dir_entry.path().is_file() {
            if let Ok(meta) = dir_entry.path().metadata() {
                size += meta.len();
            }
        }
    }

    size
}

fn has_files(path: &PathBuf, details: &mut CrateDetails) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("has_files for {path:?}");

    details.has_build_rs = path.join("build.rs").exists();

    let standard_folders = [
        OsStr::new("src"),
        OsStr::new("tests"),
        OsStr::new("examples"),
        OsStr::new("benches"),
    ];

    let folders = path
        .read_dir()?
        .flatten()
        .filter_map(|entry| {
            if !entry.path().is_dir() || standard_folders.contains(&entry.file_name().as_os_str()) {
                None
            } else {
                #[allow(clippy::option_map_or_none)]
                entry
                    .file_name()
                    .to_str()
                    .map_or(None, |file_name| Some(file_name.to_owned()))
            }
        })
        .collect::<Vec<String>>();

    log::info!("nonstandard_folders: {:?}", folders);
    details.nonstandard_folders = folders;

    Ok(())
}

fn save_details(
    details: &CrateDetails,
    filepath: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Saving crate details to {filepath:?}");
    let mut file = File::create(filepath)?;
    writeln!(&mut file, "{}", serde_json::to_string(details)?)?;

    Ok(())
}
