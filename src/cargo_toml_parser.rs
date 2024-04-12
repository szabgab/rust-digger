#![allow(dead_code)]
use serde::Deserialize;
use toml::Value;

#[derive(Deserialize, Debug)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub edition: Option<String>,

    pub rust_version: Option<String>,

    #[serde(alias = "rust-version")]
    pub rust_dash_version: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Cargo {
    pub package: Package,
    pub dependencies: Option<Value>,
}
