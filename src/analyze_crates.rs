use std::error::Error;
use std::path::PathBuf;

use clap::Parser;

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
        help = "Limit the number of crates we process."
    )]
    limit: usize,
}

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    log::info!("Start analyzing crates.");
    let start_time = std::time::Instant::now();

    if let Err(err) = run() {
        log::error!("Error: {err}");
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
        log::info!("We are going to process only {limit} crates");
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
        log::info!("{dir_entry:?}");

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
        details.has_files(&dir_entry.path())?;
        log::info!("details: {details:#?}");
        details.disk_size(&dir_entry.path());
        details.save(filepath)?;
        crate_details.push(details);
    }

    std::fs::write(
        get_data_folder().join("crate_details.json"),
        serde_json::to_vec(&crate_details)?,
    )?;

    Ok(())
}
