#![allow(dead_code)]

use std::error::Error;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use toml::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub edition: Option<String>,
    pub authors: Option<Vec<String>>,
    pub description: Option<String>,
    pub readme: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub build: Option<Value>,
    //pub build: Option<String>,
    //pub build: Option<bool>,
    pub resolver: Option<String>,
    pub links: Option<String>,

    #[serde(alias = "default-run")]
    pub default_dash_run: Option<String>,

    // Some Crates use rust_version some crates use rust-version:
    #[serde(alias = "rust-version")]
    pub rust_dash_version: Option<String>,
    pub rust_version: Option<String>,

    #[serde(alias = "license-file")]
    pub license_dash_file: Option<String>,

    #[serde(alias = "license_file")]
    pub license_file: Option<String>,

    #[serde(alias = "licenseFile")]
    pub license_capital_file: Option<String>,

    #[serde(alias = "forced-target")]
    pub forced_dash_target: Option<String>,

    pub autobins: Option<bool>,
    pub autotests: Option<bool>,
    pub autoexamples: Option<bool>,
    pub autobenches: Option<bool>,

    pub publish: Option<Value>,
    //pub publish: Option<bool>,
    //pub publish: Option<Vec<String>>,
    pub metadata: Option<Value>,
    pub keywords: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cargo {
    pub package: Package,
    pub dependencies: Option<Value>,
}

pub fn load_cargo_toml(path: &PathBuf) -> Result<Cargo, Box<dyn Error>> {
    log::debug!("load_cargo_toml {:?}", path);
    let content = std::fs::read_to_string(path)?;
    let parsed: Cargo = toml::from_str(&content)?;
    Ok(parsed)
}
