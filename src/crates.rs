use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use rust_digger::{crates_root, RealCargo, RealCrate};

mod cargo_toml_parser;
use cargo_toml_parser::Cargo;

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    log::info!("Start analyzing crates");
    let start_time = std::time::Instant::now();

    fs::create_dir_all(crates_root()).unwrap();

    let Ok(dir_handle) = crates_root().read_dir() else {
        log::error!("Could not read directory {:?}", crates_root());
        return;
    };

    let mut crates: Vec<RealCrate> = vec![];
    for entry in dir_handle.flatten() {
        let path = entry.path();
        log::info!("Processing {:?}", path);

        match process_cargo_toml(&path.join("Cargo.toml")) {
            Ok(cargo) => crates.push(RealCrate { cargo }),
            Err(err) => log::error!("{err}"),
        }
    }

    let filename = "crates.json";
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{}", serde_json::to_string(&crates).unwrap()).unwrap();

    log::info!("Elapsed time: {} sec.", start_time.elapsed().as_secs());
    log::info!("End analyzing crates");
}

fn process_cargo_toml(path: &PathBuf) -> Result<RealCargo, Box<dyn std::error::Error>> {
    let cargo = load_cargo_toml(path)?;
    //let package = cargo_toml.get("package").ok_or("Could not find package")?;
    //let x = package.get("name").ok_or("No name in Cargo.toml")?.as_str().ok;

    //let cargo: RealCargo = serde_toml::from_str(&text)

    let real_cargo = RealCargo {
        name: cargo.package.name,
        version: cargo.package.version, // package.get("version").ok_or("No version in Cargo.toml")?,
    };

    Ok(real_cargo)
    // edition
    // rust-version
    // rust_version
}

// println!("edition: {:?}", parsed.package.edition);
// println!("rust_version: {:?}", parsed.package.rust_version);
// println!("rust-version: {:?}", parsed.package.rust_dash_version);

// println!("dependencies: {:?}", parsed.dependencies);

fn load_cargo_toml(path: &PathBuf) -> Result<Cargo, Box<dyn Error>> {
    let content = std::fs::read_to_string(path)?;
    let parsed: Cargo = toml::from_str(&content)?;
    Ok(parsed)
}
