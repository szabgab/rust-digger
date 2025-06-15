#![allow(clippy::pub_use)]

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::read_to_string;
use std::fs::File;
use std::io::Write as _;
use std::path::PathBuf;

use git_digger::Repository;

mod cargo_toml_parser;
pub use cargo_toml_parser::{load_cargo_toml, load_name_version_toml, Cargo};

mod timer;
pub use timer::ElapsedTimer;

#[expect(clippy::struct_excessive_bools)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct CrateDetails {
    pub has_build_rs: bool,
    pub has_cargo_toml: bool,
    pub has_cargo_lock: bool,
    pub has_clippy_toml: bool,
    pub has_dot_clippy_toml: bool,
    pub has_rustfmt_toml: bool,
    pub has_dot_rustfmt_toml: bool,
    pub has_main_rs: bool,
    pub nonstandard_folders: Vec<String>,
    pub size: u64,
}

impl CrateDetails {
    pub const fn new() -> Self {
        Self {
            has_build_rs: false,
            has_cargo_toml: false,
            has_cargo_lock: false,
            has_clippy_toml: false,
            has_dot_clippy_toml: false,
            has_rustfmt_toml: false,
            has_dot_rustfmt_toml: false,
            has_main_rs: false,
            nonstandard_folders: vec![],
            size: 0,
        }
    }
}

impl Default for CrateDetails {
    fn default() -> Self {
        Self::new()
    }
}

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
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[expect(clippy::struct_excessive_bools)]
pub struct VCSDetails {
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
}

impl VCSDetails {
    pub const fn new() -> Self {
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
        }
    }
}

impl Default for VCSDetails {
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
    pub vcs_details: VCSDetails,

    #[serde(default = "empty_cargo")]
    pub cargo: Cargo,

    #[serde(default = "empty_crate_details")]
    pub crate_details: CrateDetails,
}

impl Crate {
    pub const fn new() -> Self {
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

            vcs_details: VCSDetails::new(),
            cargo: Cargo::new(),
            crate_details: CrateDetails::new(),
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
pub type CrateErrors = HashMap<String, String>;
pub type CargoTomlErrors = HashMap<String, String>;
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

const fn empty_details() -> VCSDetails {
    VCSDetails::new()
}

const fn empty_cargo() -> Cargo {
    Cargo::new()
}

const fn empty_crate_details() -> CrateDetails {
    CrateDetails::new()
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

pub fn analyzed_crates_root() -> PathBuf {
    get_data_folder().join("analyzed-crates")
}

pub fn repo_details_root() -> PathBuf {
    get_data_folder().join("repo-details")
}

pub fn collected_data_root() -> PathBuf {
    get_data_folder().join("collected-data")
}

/// Creates the data folders we need if they do not exist.
pub fn create_data_folders() -> Result<(), Box<dyn Error>> {
    if !get_data_folder().exists() {
        fs::create_dir_all(get_data_folder())?;
    }
    fs::create_dir_all(get_repos_folder())?;
    fs::create_dir_all(get_db_dump_folder())?;
    fs::create_dir_all(get_temp_folder())?;
    fs::create_dir_all(crates_root()).unwrap();
    fs::create_dir_all(analyzed_crates_root()).unwrap();

    Ok(())
}

pub fn percentage(num: usize, total: usize) -> String {
    let total_f32 = (10000.0 * num as f32 / total as f32).floor();
    (total_f32 / 100.0).to_string()
}

pub fn get_vcs_details_path(url: &str) -> Option<PathBuf> {
    let repository = Repository::from_url(url);
    if url.is_empty() {
        log::warn!("Repository URL is empty, not saving details");
        return None;
    }
    match repository {
        Ok(repo) => {
            let mut details_path = repo.path(repo_details_root().as_path());
            details_path.set_extension("json");
            Some(details_path)
        }
        Err(err) => {
            log::error!("Error parsing repository URL in get_vcs_details_path: {err}");
            None
        }
    }
}

pub fn load_vcs_details(repository: &str) -> VCSDetails {
    //let _a = ElapsedTimer::new("load_vcs_details");
    log::info!("Load details started for {repository}");

    let Some(details_path) = get_vcs_details_path(repository) else {
        return VCSDetails::new();
    };

    if !details_path.exists() {
        return VCSDetails::new();
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
                    return VCSDetails::new();
                }
            };
        }
        Err(error) => {
            log::error!("Error opening file {}: {}", details_path.display(), error);
        }
    }
    VCSDetails::new()
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
pub fn save_details(repository: &str, details: &VCSDetails) -> Result<(), Box<dyn Error>> {
    log::info!("save_details for '{repository}'");

    create_repo_details_folders()?;

    if repository.is_empty() {
        log::warn!("Repository URL is empty, not saving details");
        return Ok(());
    }

    match Repository::from_url(repository) {
        Ok(repo) => {
            let _res = fs::create_dir_all(repo.owner_path(repo_details_root().as_path()));
            let mut details_path = repo.path(repo_details_root().as_path());
            details_path.set_extension("json");
            // log::info!("details {:#?}", &details);
            log::info!(
                "Going to save in details_path {:?}",
                &details_path.display()
            );
            // if Path::new(&details_path).exists() {
            //     mqatch File::open(details_path.to_string()) {
            // }

            let content = serde_json::to_string(&details).unwrap();
            let mut file = File::create(details_path).unwrap();
            writeln!(&mut file, "{content}").unwrap();

            Ok(())
        }
        Err(err) => {
            log::error!("Error parsing repository URL in save_details: {err}");
            Ok(()) // this should never happen
        }
    }
}

pub fn load_cargo_toml_released_crates(
) -> Result<(Vec<Cargo>, CrateErrors, CargoTomlErrors), Box<dyn Error>> {
    let released_crates = serde_json::from_str(&read_to_string(
        get_data_folder().join("released_cargo_toml.json"),
    )?)?;
    let released_cargo_toml_errors = serde_json::from_str(&read_to_string(
        get_data_folder().join("released_cargo_toml_errors.json"),
    )?)?;
    let released_cargo_toml_errors_nameless = serde_json::from_str(&read_to_string(
        get_data_folder().join("released_cargo_toml_errors_nameless.json"),
    )?)?;

    Ok((
        released_crates,
        released_cargo_toml_errors,
        released_cargo_toml_errors_nameless,
    ))
}

pub fn collect_cargo_toml_released_crates() -> Result<(), Box<dyn Error>> {
    let _a = ElapsedTimer::new("collect_cargo_toml_released_crates");

    let dir_handle = crates_root().read_dir()?;
    let mut released_cargo_toml_errors: CrateErrors = HashMap::new();
    let mut released_cargo_toml_errors_nameless: CargoTomlErrors = HashMap::new();

    let released_crates = dir_handle
        .flatten()
        .filter_map(|entry| {
            let path = entry.path().join("Cargo.toml");
            match load_cargo_toml(&path) {
                Ok(cargo) => Some(cargo),
                Err(err) => {
                    log::error!("Reading {:?} failed: {err}", path.display());

                    match load_name_version_toml(&path) {
                        Ok((name, _version)) => {
                            released_cargo_toml_errors.insert(name, format!("{err}"));
                        }
                        Err(err2) => {
                            released_cargo_toml_errors_nameless.insert(
                                format!("{:?}", &entry.file_name().display()),
                                format!("{err2}"),
                            );
                            log::error!(
                                "Can't load the name and version of the crate {:?} failed: {err2}",
                                path.display()
                            );
                        }
                    }

                    None
                }
            }
        })
        .collect::<Vec<Cargo>>();

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

    Ok(())
}

/// Reads the `versions.csv` file (the database dump from Crates.io) and returns a vector of `CrateVersion` structs.
/// # Errors
/// If the file desn't exist or is not a proper CSV file.
pub fn read_versions() -> Result<Vec<CrateVersion>, Box<dyn Error>> {
    let filepath = get_db_dump_folder().join("data/versions.csv");
    log::info!("Start reading {:?}", filepath.display());

    let mut versions: Vec<CrateVersion> = vec![];
    let file = File::open(&filepath)?;
    let mut rdr = csv::Reader::from_reader(file);
    for result in rdr.deserialize() {
        let record: CrateVersion = result?;
        versions.push(record);
    }

    log::info!("Finished reading {:?}", filepath.display());

    Ok(versions)
}

pub fn add_cargo_toml_to_crates(
    crates: Vec<Crate>,
) -> Result<(Vec<Crate>, CrateErrors, CargoTomlErrors), Box<dyn Error>> {
    let _a = ElapsedTimer::new("add_cargo_toml_to_crates");

    let (released_crates, released_cargo_toml_errors, released_cargo_toml_errors_nameless) =
        load_cargo_toml_released_crates()?;
    let cargo_of_crate: HashMap<String, Cargo> = released_crates
        .iter()
        .map(|krate| (krate.package.name.clone(), krate.clone()))
        .collect::<HashMap<_, _>>();

    let updated_crates = crates
        .into_iter()
        .map(|mut krate| {
            krate.cargo = if cargo_of_crate.contains_key(&krate.name) {
                cargo_of_crate[&krate.name].clone()
            } else {
                Cargo::new()
            };
            krate
        })
        .collect::<Vec<Crate>>();

    Ok((
        updated_crates,
        released_cargo_toml_errors,
        released_cargo_toml_errors_nameless,
    ))
}

/// Reads the `crates.csv` file (the database dump from Crates.io) and returns a vector of `Crate` structs.
///
/// # Errors
///
/// Will return `Err` if can't open `crates.csv` or if it is not a
/// proper CSV file.
pub fn read_crates(limit: u32) -> Result<Vec<Crate>, Box<dyn Error>> {
    let filepath = get_db_dump_folder().join("data/crates.csv");
    log::info!("Start reading {:?}", filepath.display());

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

        let krate: Crate = result?;

        crates.push(krate);
    }
    #[expect(clippy::min_ident_chars)]
    crates.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    log::info!("Finished reading {:?}", filepath.display());
    Ok(crates)
}

pub fn build_path(mut path: PathBuf, parts: &[&str], extension: Option<&str>) -> PathBuf {
    for part in parts {
        path = path.join(part);
    }

    if let Some(ext) = extension {
        path.set_extension(ext);
    }

    path
}

pub fn load_crate_details(filepath: &PathBuf) -> Result<CrateDetails, Box<dyn Error>> {
    log::info!("load_crate_details {:?}", filepath.display());
    let content = std::fs::read_to_string(filepath)?;
    Ok(serde_json::from_str::<CrateDetails>(&content)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    //use crate::repo_details_root;

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
            .join("github.com")
            .join("foo")
            .join("bar.json");
        assert_eq!(
            get_vcs_details_path("https://github.com/foo/bar")
                .expect("X")
                .as_path(),
            expected
        );

        assert_eq!(
            get_vcs_details_path("https://github.com/foo/bar/baz")
                .expect("X")
                .as_path(),
            expected
        ); // TODO this should not work I think
        assert_eq!(get_vcs_details_path("https://zorg.com/foo/bar"), None);
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
