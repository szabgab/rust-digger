#![allow(dead_code)]

use std::collections::HashMap;
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

impl Package {
    pub const fn new() -> Self {
        Self {
            name: String::new(),
            version: String::new(),
            edition: None,
            authors: None,
            description: None,
            readme: None,
            license: None,
            repository: None,
            homepage: None,
            documentation: None,
            build: None,

            resolver: None,
            links: None,

            default_dash_run: None,

            rust_dash_version: None,
            rust_version: None,

            license_dash_file: None,

            license_file: None,

            license_capital_file: None,

            forced_dash_target: None,

            autobins: None,
            autotests: None,
            autoexamples: None,
            autobenches: None,

            publish: None,
            metadata: None,
            keywords: None,
            categories: None,
            exclude: None,
            include: None,
        }
    }
}

impl Default for Package {
    fn default() -> Self {
        Self::new()
    }
}

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct CargoDependencyValue {
//     pub optional: Option<bool>,
//     pub version: String,
// }
// CargoDependencyValue

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cargo {
    pub package: Package,
    pub dependencies: Option<HashMap<String, Value>>,
}

impl Cargo {
    pub const fn new() -> Self {
        Self {
            package: Package::new(),
            dependencies: None,
        }
    }
}

impl Default for Cargo {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimplePackage {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimpleCargo {
    pub package: SimplePackage,
    pub dependencies: Option<Value>,
}

pub fn load_cargo_toml(path: &PathBuf) -> Result<Cargo, Box<dyn Error>> {
    log::debug!("load_cargo_toml {:?}", path.display());
    let content = std::fs::read_to_string(path)?;
    let parsed: Cargo = toml::from_str(&content)?;
    Ok(parsed)
}

pub fn load_name_version_toml(path: &PathBuf) -> Result<(String, String), Box<dyn Error>> {
    log::debug!("load_name_version_toml {:?}", path.display());
    let content = std::fs::read_to_string(path)?;
    let parsed: SimpleCargo = toml::from_str(&content)?;
    Ok((parsed.package.name, parsed.package.version))
}
