use std::error::Error;
use std::path::PathBuf;
use std::{collections::HashMap, vec};

use clap::Parser;

use rust_digger::{
    analyzed_crates_root, crates_root, create_data_folders, get_data_folder, CargoTomlErrors,
    CrateDetails, CrateErrors, ElapsedTimer,
};

mod cargo_toml_parser;
use cargo_toml_parser::{load_cargo_toml, load_cargo_toml_simplified, Cargo};

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
    let mut released_cargo_toml_errors: CrateErrors = HashMap::new();
    let mut released_cargo_toml_errors_nameless: CargoTomlErrors = HashMap::new();
    let mut released_cargo_toml_in_lower_case: Vec<String> = vec![];
    let mut released_cargo_toml_missing: Vec<String> = vec![];
    let mut released_crates: Vec<Cargo> = vec![];

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

        // For now we disable this optimization. The total procsessing time is not that long (400 sec)
        // and as it is now, the skipping here also skips the loading of the Cargo.toml file
        // which is needed for the analysis.

        // try to read the already collected data, if it succeeds go to the next crate
        // if let Ok(content) = std::fs::read_to_string(&filepath) {
        //     if let Ok(_details) = serde_json::from_str::<CrateDetails>(&content) {
        //         log::info!("Details found");
        //         continue;
        //     }
        // }

        // if it fails collect all the data and save to the disk
        let mut details = CrateDetails::new();
        details.has_files(&dir_entry.path())?;
        log::info!("details: {details:#?}");
        details.disk_size(&dir_entry.path());
        details.save(filepath)?;

        let path_or_none = if details.has_cargo_toml {
            Some(dir_entry.path().join("Cargo.toml"))
        } else if details.has_cargo_toml_in_lower_case {
            //released_cargo_toml_in_lower_case.push(dir_entry.file_name().display().to_string());
            Some(dir_entry.path().join("cargo.toml"))
        } else {
            None
        };

        if let Some(path) = path_or_none {
            match load_cargo_toml(&path) {
                Ok(cargo) => {
                    if details.has_cargo_toml_in_lower_case {
                        released_cargo_toml_in_lower_case.push(cargo.package.name.clone());
                    }
                    released_crates.push(cargo.clone());
                }
                Err(err) => {
                    log::warn!("Reading Cargo.toml {:?} failed: {err}", path.display());

                    match load_cargo_toml_simplified(&path) {
                        Ok((name, _version)) => {
                            released_cargo_toml_errors.insert(name, format!("{err}"));
                        }
                        Err(err2) => {
                            released_cargo_toml_errors_nameless.insert(
                                format!("{:?}", &dir_entry.file_name().display()),
                                format!("{err2}"),
                            );
                            log::error!(
                                "Can't load the name and version of the crate {:?} failed: {err2}",
                                path.display()
                            );
                        }
                    }
                }
            }
        } else {
            log::warn!("No Cargo.toml found in {:?}", dir_entry.path().display());
            released_cargo_toml_missing.push(dir_entry.file_name().display().to_string());
        }

        crate_details.push(details);
    }

    std::fs::write(
        get_data_folder().join("crate_details.json"),
        serde_json::to_vec(&crate_details)?,
    )?;
    std::fs::write(
        get_data_folder().join("released_cargo_toml.json"),
        serde_json::to_vec(&released_crates)?,
    )?;
    std::fs::write(
        get_data_folder().join("released_cargo_toml_errors.json"),
        serde_json::to_vec(&released_cargo_toml_errors)?,
    )?;
    std::fs::write(
        get_data_folder().join("released_cargo_toml_errors_nameless.json"),
        serde_json::to_vec(&released_cargo_toml_errors_nameless)?,
    )?;

    std::fs::write(
        get_data_folder().join("released_cargo_toml_missing.json"),
        serde_json::to_vec(&released_cargo_toml_missing)?,
    )?;

    std::fs::write(
        get_data_folder().join("released_cargo_toml_in_lower_case.json"),
        serde_json::to_vec(&released_cargo_toml_in_lower_case)?,
    )?;

    Ok(())
}
