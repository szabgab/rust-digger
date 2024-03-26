use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Details {
    pub has_github_action: bool,
    pub has_gitlab_pipeline: bool,
    pub commit_count: i32,
    pub cargo_toml_in_root: bool,
    pub cargo_fmt: String,

    #[serde(default = "empty_string")]
    pub git_clone_error: String,
}

impl Details {
    pub fn new() -> Details {
        Details {
            has_github_action: false,
            has_gitlab_pipeline: false,
            commit_count: 0,
            cargo_toml_in_root: false,
            cargo_fmt: String::new(),

            git_clone_error: String::new(),
        }
    }
}

impl Default for Details {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Repo {
    pub display: String,
    pub name: String,
    pub url: String,
    pub count: usize,
    pub percentage: String,
    pub crates: Vec<Crate>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Crate {
    pub created_at: String,
    pub description: String,
    pub documentation: String,
    pub downloads: String,
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
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub gh_avatar: String,
    pub gh_id: String,
    pub gh_login: String,
    pub id: String,
    pub name: String,

    #[serde(default = "get_zero")]
    pub count: u16,
}

fn empty_details() -> Details {
    Details::new()
}

fn empty_string() -> String {
    String::new()
}

fn get_zero() -> u16 {
    0
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

impl Crate {
    pub fn new() -> Crate {
        Crate {
            created_at: String::new(),
            description: String::new(),
            documentation: String::new(),
            downloads: String::new(),
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
        }
    }
}
impl Default for Crate {
    fn default() -> Self {
        Self::new()
    }
}

//type RepoPercentage<'a> = HashMap<&'a str, String>;
pub type Owners = HashMap<String, String>;
pub type CratesByOwner = HashMap<String, Vec<String>>;
// type Users = HashMap<String, User>;

pub fn get_owner_and_repo(repository: &str) -> (String, String, String) {
    static RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^https://(github|gitlab).com/([^/]+)/([^/]+)/?.*$").unwrap());
    let repo_url = match RE.captures(repository) {
        Some(value) => value,
        None => {
            log::warn!("No match for repo in {}", &repository);
            return (String::new(), String::new(), String::new());
        }
    };
    let host = repo_url[1].to_lowercase();
    let owner = repo_url[2].to_lowercase();
    let repo = repo_url[3].to_lowercase();
    (host, owner, repo)
}

pub fn percentage(num: usize, total: usize) -> String {
    let total = 10000.0 * num as f32 / total as f32;
    (total / 100.0).to_string()
}

pub fn repo_details_root() -> String {
    "repo-details".to_string()
}

pub fn get_details_path(repository: &str) -> Option<PathBuf> {
    let (host, owner, repo) = get_owner_and_repo(repository);
    if repo.is_empty() {
        return None;
    }

    let mut details_path = PathBuf::new();
    details_path.push(format!(
        "{}/{host}/{owner}/{repo}.json",
        repo_details_root()
    ));
    Some(details_path)
}

pub fn load_details(repository: &str) -> Details {
    log::info!("Load details started for {}", repository);

    let details_path = match get_details_path(repository) {
        Some(val) => val,
        None => return Details::new(),
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
            println!("Error opening file {}: {}", details_path.display(), error);
        }
    }
    Details::new()
}

pub fn save_details(repository: &str, details: &Details) {
    log::info!("save_details for '{}'", repository);

    let _res = fs::create_dir_all(repo_details_root());
    let _res = fs::create_dir_all(format!("{}/github", repo_details_root()));
    let _res = fs::create_dir_all(format!("{}/gitlab", repo_details_root()));

    let (host, owner, repo) = get_owner_and_repo(repository);
    if owner.is_empty() {
        return; // this should never happen
    }

    let _res = fs::create_dir_all(format!("{}/{host}/{owner}", repo_details_root()));
    let details_path = format!("{}/{host}/{owner}/{repo}.json", repo_details_root());
    // if Path::new(&details_path).exists() {
    //     match File::open(details_path.to_string()) {
    // }

    let content = serde_json::to_string(&details).unwrap();
    let mut file = File::create(details_path).unwrap();
    writeln!(&mut file, "{}", content).unwrap();
}

pub fn read_crates(limit: u32) -> Vec<Crate> {
    let filepath = "data/data/crates.csv";
    log::info!("Start reading {}", filepath);
    let mut crates: Vec<Crate> = vec![];
    let mut count = 0;
    match File::open(filepath) {
        Ok(file) => {
            let mut rdr = csv::Reader::from_reader(file);
            for result in rdr.deserialize() {
                count += 1;
                if limit > 0 && count >= limit {
                    log::info!("Limit of {limit} reached");
                    break;
                }
                let record: Crate = match result {
                    Ok(value) => value,
                    Err(error) => panic!("error: {}", error),
                };
                crates.push(record);
            }
            #[allow(clippy::min_ident_chars)]
            crates.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        }
        Err(error) => panic!("Error opening file {}: {}", filepath, error),
    }

    log::info!("Finished reading {filepath}");
    crates
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
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
        assert_eq!(
            get_details_path("https://github.com/foo/bar")
                .expect("X")
                .as_path(),
            Path::new(&format!("{}/github/foo/bar.json", repo_details_root()))
        );
        assert_eq!(
            get_details_path("https://github.com/foo/bar/baz")
                .expect("X")
                .as_path(),
            Path::new(&format!("{}/github/foo/bar.json", &repo_details_root()))
        ); // TODO this should not work I think
        assert_eq!(get_details_path("https://zorg.com/foo/bar"), None);
    }
}
