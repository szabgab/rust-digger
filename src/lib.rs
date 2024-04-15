#![allow(clippy::pub_use)]

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use once_cell::sync::Lazy;
use regex::Regex;

mod cargo_toml_parser;
pub use cargo_toml_parser::{load_cargo_toml, Cargo};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum RepoPlatform {
    GitHub,    // https://github.com/
    GitLab,    // https://gitlab.com/
    Gitea,     // https://about.gitea.com/
    Cgit,      // https://git.zx2c4.com/cgit/about/
    Forgejo,   // https://forgejo.org/
    Fossil,    // https://fossil-scm.org/
    Mercurial, // https://www.mercurial-scm.org/
    Gogs,      // https://gogs.io/
}

const REPO_FOLDERS: [&str; 2] = ["github", "gitlab"];
const URL_REGEXES: [&str; 2] = [
    "^https://(github).com/([^/]+)/([^/]+)/?.*$",
    "^https://(gitlab).com/([^/]+)/([^/]+)/?.*$",
];

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct Details {
    #[serde(default = "default_false")]
    pub has_github_action: bool,
    #[serde(default = "default_false")]
    pub has_gitlab_pipeline: bool,
    #[serde(default = "default_false")]
    pub has_circle_ci: bool,
    #[serde(default = "default_false")]
    pub has_cirrus_ci: bool,
    #[serde(default = "default_false")]
    pub has_travis_ci: bool,
    #[serde(default = "default_false")]
    pub has_jenkins: bool,
    #[serde(default = "default_false")]
    pub has_appveyor: bool,
    #[serde(default = "default_false")]
    pub has_azure_pipeline: bool,
    #[serde(default = "default_false")]
    pub has_bitbucket_pipeline: bool,

    pub commit_count: i32,
    pub cargo_toml_in_root: bool,
    pub cargo_fmt: String,

    #[serde(default = "empty_string")]
    pub git_clone_error: String,

    #[serde(default = "default_false")]
    pub has_rustfmt_toml: bool,

    #[serde(default = "default_false")]
    pub has_dot_rustfmt_toml: bool,

    #[serde(default = "empty_string")]
    pub edition: String,

    #[serde(default = "empty_string")]
    pub rust_version: String,

    #[serde(default = "empty_string")]
    pub rust_dash_version: String,
}

impl Details {
    pub fn new() -> Self {
        Self {
            has_github_action: false,
            has_gitlab_pipeline: false,
            has_circle_ci: false,
            has_cirrus_ci: false,
            has_travis_ci: false,
            has_jenkins: false,
            has_appveyor: false,
            has_azure_pipeline: false,
            has_bitbucket_pipeline: false,

            commit_count: 0,
            cargo_toml_in_root: false,
            cargo_fmt: String::new(),
            has_rustfmt_toml: false,
            has_dot_rustfmt_toml: false,

            git_clone_error: String::new(),
            edition: String::new(),
            rust_version: String::new(),
            rust_dash_version: String::new(),
        }
    }
}

impl Default for Details {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repo {
    pub display: String,
    pub name: String,
    pub url: String,

    #[serde(default = "get_default_count")]
    pub count: usize,

    #[serde(default = "get_default_percentage")]
    pub percentage: String,

    pub platform: Option<RepoPlatform>,

    #[serde(default = "get_default_bold")]
    pub bold: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct CrateVersion {
    pub checksum: String,
    pub crate_id: String,
    pub crate_size: String,
    pub created_at: String,
    pub features: String,
    pub id: String,
    pub license: String,
    pub links: String,
    pub num: String,
    pub published_by: String,
    pub rust_version: String,
    pub updated_at: String,
    pub yanked: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Crate {
    pub created_at: String,
    pub description: String,
    pub documentation: String,
    pub homepage: String,
    pub id: String,
    pub max_upload_size: String,
    pub name: String,
    pub readme: String,
    pub repository: String,
    pub updated_at: String,

    #[serde(default = "empty_string")]
    pub owner_gh_login: String,

    #[serde(default = "empty_string")]
    pub owner_name: String,

    #[serde(default = "empty_string")]
    pub owner_gh_avatar: String,

    #[serde(default = "empty_details")]
    pub details: Details,

    pub cargo: Option<Cargo>,
}

impl Crate {
    pub fn new() -> Self {
        Self {
            created_at: String::new(),
            description: String::new(),
            documentation: String::new(),
            homepage: String::new(),
            id: String::new(),
            max_upload_size: String::new(),
            name: String::new(),
            readme: String::new(),
            repository: String::new(),
            updated_at: String::new(),

            owner_gh_avatar: String::new(),
            owner_gh_login: String::new(),
            owner_name: String::new(),

            details: Details::new(),
            cargo: None,
        }
    }
}
impl Default for Crate {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub gh_avatar: String,
    pub gh_id: String,
    pub gh_login: String,
    pub id: String,
    pub name: String,

    #[serde(default = "get_zero")]
    pub count: usize,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Team {
    pub avatar: String,
    pub github_id: String,
    pub login: String,
    pub id: String,
    pub name: String,
    pub org_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CrateOwner {
    pub crate_id: String,
    pub created_at: String,
    pub created_by: String,
    pub owner_id: String,
    pub owner_kind: String,
}

//type RepoPercentage<'a> = HashMap<&'a str, String>;
pub type Owners = HashMap<String, String>;
pub type CratesByOwner = HashMap<String, Vec<String>>;
// type Users = HashMap<String, User>;

const fn get_default_bold() -> bool {
    false
}

const fn get_default_count() -> usize {
    0
}

fn get_default_percentage() -> String {
    String::from("0")
}

fn empty_details() -> Details {
    Details::new()
}

const fn empty_string() -> String {
    String::new()
}

const fn get_zero() -> usize {
    0
}

const fn default_false() -> bool {
    false
}

pub fn get_data_folder() -> PathBuf {
    PathBuf::from("data")
}

pub fn get_repos_folder() -> PathBuf {
    get_data_folder().join("repos")
}

pub fn get_db_dump_folder() -> PathBuf {
    get_data_folder().join("db-dump")
}

pub fn get_temp_folder() -> PathBuf {
    get_data_folder().join("temp")
}

pub fn crates_root() -> PathBuf {
    get_data_folder().join("crates")
}

pub fn repo_details_root() -> PathBuf {
    get_data_folder().join("repo-details")
}

pub fn collected_data_root() -> PathBuf {
    get_data_folder().join("collected-data")
}

pub fn create_data_folders() -> Result<(), Box<dyn Error>> {
    if !get_data_folder().exists() {
        fs::create_dir_all(get_data_folder())?;
    }
    fs::create_dir_all(get_repos_folder())?;
    fs::create_dir_all(get_db_dump_folder())?;
    fs::create_dir_all(get_temp_folder())?;
    fs::create_dir_all(crates_root()).unwrap();

    Ok(())
}

pub fn get_owner_and_repo(repository: &str) -> (String, String, String) {
    static REGS: Lazy<Vec<Regex>> = Lazy::new(|| {
        URL_REGEXES
            .iter()
            .map(|reg| Regex::new(reg).unwrap())
            .collect::<Vec<Regex>>()
    });

    for re in REGS.iter() {
        if let Some(repo_url) = re.captures(repository) {
            let host = repo_url[1].to_lowercase();
            let owner = repo_url[2].to_lowercase();
            let repo = repo_url[3].to_lowercase();
            return (host, owner, repo);
        }
    }

    log::warn!("No match for repo in '{}'", &repository);
    (String::new(), String::new(), String::new())
}

pub fn percentage(num: usize, total: usize) -> String {
    let total_f32 = (10000.0 * num as f32 / total as f32).floor();
    (total_f32 / 100.0).to_string()
}

pub fn get_details_path(repository: &str) -> Option<PathBuf> {
    let (host, owner, repo) = get_owner_and_repo(repository);

    if repo.is_empty() {
        return None;
    }

    let details_path = build_path(repo_details_root(), &[&host, &owner, &repo], Some("json"));
    Some(details_path)
}

pub fn load_details(repository: &str) -> Details {
    log::info!("Load details started for {}", repository);

    let Some(details_path) = get_details_path(repository) else {
        return Details::new();
    };

    if !details_path.exists() {
        return Details::new();
    }

    match File::open(&details_path) {
        Ok(file) => {
            match serde_json::from_reader(file) {
                Ok(details) => return details,
                Err(err) => {
                    log::error!(
                        "Error reading details from '{}' {}",
                        details_path.display(),
                        err
                    );
                    return Details::new();
                }
            };
        }
        Err(error) => {
            log::error!("Error opening file {}: {}", details_path.display(), error);
        }
    }
    Details::new()
}

fn create_repo_details_folders() -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(repo_details_root())?;
    for folder in REPO_FOLDERS {
        fs::create_dir_all(repo_details_root().join(folder))?;
    }

    Ok(())
}

/// # Errors
///
/// Will return Err if can't create folders.
pub fn save_details(repository: &str, details: &Details) -> Result<(), Box<dyn Error>> {
    log::info!("save_details for '{repository}'");

    create_repo_details_folders()?;

    let (host, owner, repo) = get_owner_and_repo(repository);
    if owner.is_empty() {
        return Ok(()); // this should never happen
    }

    let _res = fs::create_dir_all(repo_details_root().join(&host).join(&owner));
    let details_path = build_path(repo_details_root(), &[&host, &owner, &repo], Some("json"));
    // log::info!("details {:#?}", &details);
    log::info!("Going to save in details_path {:?}", &details_path);
    // if Path::new(&details_path).exists() {
    //     match File::open(details_path.to_string()) {
    // }

    let content = serde_json::to_string(&details).unwrap();
    let mut file = File::create(details_path).unwrap();
    writeln!(&mut file, "{content}").unwrap();

    Ok(())
}

pub fn load_released_crates() -> Result<Vec<Cargo>, Box<dyn Error>> {
    let dir_handle = crates_root().read_dir()?;

    let released_crates = dir_handle
        .flatten()
        .filter_map(|entry| {
            let path = entry.path().join("Cargo.toml");
            log::info!("Processing {:?}", path);
            match load_cargo_toml(&path) {
                Ok(cargo) => Some(cargo),
                Err(err) => {
                    log::error!("Reading {path:?} failed: {err}");
                    None
                }
            }
        })
        .collect::<Vec<Cargo>>();

    Ok(released_crates)
}

/// # Errors
/// TODO
pub fn read_versions() -> Result<Vec<CrateVersion>, Box<dyn Error>> {
    let filepath = get_db_dump_folder().join("data/versions.csv");
    log::info!("Start reading {filepath:?}");

    let mut versions: Vec<CrateVersion> = vec![];
    let file = File::open(&filepath)?;
    let mut rdr = csv::Reader::from_reader(file);
    for result in rdr.deserialize() {
        let record: CrateVersion = result?;
        versions.push(record);
    }

    log::info!("Finished reading {filepath:?}");

    Ok(versions)
}

/// # Errors
///
/// Will return `Err` if can't open `crates.csv` or if it is not a
/// proper CSV file.
pub fn read_crates(limit: u32) -> Result<Vec<Crate>, Box<dyn Error>> {
    let filepath = get_db_dump_folder().join("data/crates.csv");
    log::info!("Start reading {filepath:?}");

    let released_crates = load_released_crates()?;
    let cargo_of_crate: HashMap<String, Cargo> = released_crates
        .iter()
        .map(|krate| (krate.package.name.clone(), krate.clone()))
        .collect::<HashMap<_, _>>();

    let mut crates: Vec<Crate> = vec![];
    let mut count = 0;
    let file = File::open(&filepath)?;
    let mut rdr = csv::Reader::from_reader(file);
    for result in rdr.deserialize() {
        count += 1;
        if limit > 0 && count >= limit {
            log::info!("Limit of {limit} reached");
            break;
        }

        let mut krate: Crate = result?;

        krate.cargo = cargo_of_crate
            .contains_key(&krate.name)
            .then(|| cargo_of_crate[&krate.name].clone());

        crates.push(krate);
    }
    #[allow(clippy::min_ident_chars)]
    crates.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    log::info!("Finished reading {filepath:?}");
    Ok(crates)
}

pub fn build_path(mut path: PathBuf, parts: &[&str], extension: Option<&str>) -> PathBuf {
    for part in parts {
        path = path.join(part);
    }

    if let Some(ext) = extension {
        path.set_extension(ext);
    };

    path
}

#[cfg(test)]
mod tests {
    use super::*;
    //use crate::repo_details_root;

    #[test]
    fn test_get_owner_and_repo() {
        assert_eq!(
            get_owner_and_repo("https://github.com/szabgab/rust-digger"),
            (
                "github".to_string(),
                "szabgab".to_string(),
                "rust-digger".to_string()
            )
        );
        assert_eq!(
            get_owner_and_repo("https://github.com/szabgab/rust-digger/"),
            (
                "github".to_string(),
                "szabgab".to_string(),
                "rust-digger".to_string()
            )
        );
        assert_eq!(
            get_owner_and_repo(
                "https://github.com/crypto-crawler/crypto-crawler-rs/tree/main/crypto-market-type"
            ),
            (
                "github".to_string(),
                "crypto-crawler".to_string(),
                "crypto-crawler-rs".to_string()
            )
        );
        assert_eq!(
            get_owner_and_repo("https://gitlab.com/szabgab/rust-digger"),
            (
                "gitlab".to_string(),
                "szabgab".to_string(),
                "rust-digger".to_string()
            )
        );
        assert_eq!(
            get_owner_and_repo("https://gitlab.com/Szabgab/Rust-digger/"),
            (
                "gitlab".to_string(),
                "szabgab".to_string(),
                "rust-digger".to_string()
            )
        );
    }

    #[test]
    fn test_percentage() {
        assert_eq!(percentage(20, 100), "20");
        assert_eq!(percentage(5, 20), "25");
        assert_eq!(percentage(1234, 10000), "12.34");
        assert_eq!(percentage(1_234_567, 10_000_000), "12.34");
    }

    #[test]
    fn test_get_details_path() {
        let expected = repo_details_root()
            .join("github")
            .join("foo")
            .join("bar.json");
        assert_eq!(
            get_details_path("https://github.com/foo/bar")
                .expect("X")
                .as_path(),
            expected
        );

        assert_eq!(
            get_details_path("https://github.com/foo/bar/baz")
                .expect("X")
                .as_path(),
            expected
        ); // TODO this should not work I think
        assert_eq!(get_details_path("https://zorg.com/foo/bar"), None);
    }

    #[test]
    fn check_build_path() {
        // empty
        let path = build_path(PathBuf::from("root"), &[], None);
        assert_eq!(path, PathBuf::from("root"));

        let path = build_path(PathBuf::from("root"), &[], Some("rs"));
        assert_eq!(path, PathBuf::from("root.rs"));

        let path = build_path(PathBuf::from("root"), &["one", "two"], None);
        let mut expected = PathBuf::from("root").join("one").join("two");
        assert_eq!(path, expected);

        let path = build_path(PathBuf::from("root"), &["one", "two"], Some("html"));
        expected.set_extension("html");
        assert_eq!(path, expected);
    }
}
