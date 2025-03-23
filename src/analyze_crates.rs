use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Write as _;
use std::path::PathBuf;

use clap::Parser;
use walkdir::WalkDir;

use rust_digger::{
    analyzed_crates_root, collect_cargo_toml_released_crates, crates_root, create_data_folders,
    get_data_folder, CrateDetails, ElapsedTimer,
};

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
    let _a = ElapsedTimer::new("analyze_crates");

    let args = Cli::parse();
    log::info!("Limit: {}", args.limit);

    collect_data_from_crates(args.limit)?;
    collect_cargo_toml_released_crates()?;

    Ok(())
}

fn collect_data_from_crates(limit: usize) -> Result<(), Box<dyn std::error::Error>> {
    let _a = ElapsedTimer::new("collect_data_from_crates");

    if 0 < limit {
        log::info!("We are going to process only {} crates", limit);
    } else {
        log::info!("We are going to process all the crates we find locally",);
    }
    create_data_folders()?;
    let mut crate_details = vec![];

    for (count, entry) in crates_root().read_dir()?.enumerate() {
        if limit > 0 && count >= limit {
            break;
        }
        let dir_entry = entry?;
        log::info!("{:?}", dir_entry);

        let filepath = if let Some(crate_dirname) = dir_entry.path().file_name() {
            let filepath = analyzed_crates_root().join(crate_dirname);
            // can't use set_extension as there are dots in the names and this would remove them
            PathBuf::from(format!("{}.json", filepath.display()))
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
        crate_details.push(details);
    }

    std::fs::write(
        get_data_folder().join("crate_details.json"),
        serde_json::to_vec(&crate_details)?,
    )?;

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
    details.has_cargo_toml = path.join("Cargo.toml").exists();
    details.has_cargo_lock = path.join("Cargo.lock").exists();
    details.has_clippy_toml = path.join("clippy.toml").exists();
    details.has_dot_clippy_toml = path.join(".clippy.toml").exists();
    details.has_rustfmt_toml = path.join("rustfmt.toml").exists();
    details.has_dot_rustfmt_toml = path.join(".rustfmt.toml").exists();
    details.has_main_rs = path.join("src/main.rs").exists();

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
                #[expect(clippy::option_map_or_none)]
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
