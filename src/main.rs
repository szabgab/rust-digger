use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

use clap::Parser;

pub type Partials = liquid::partials::EagerCompiler<liquid::partials::InMemorySource>;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PAGE_SIZE: usize = 100;

mod common;
use common::{get_owner_and_repo, percentage};
mod read;
use read::{read_crate_owners, read_crates, read_teams, read_users};
mod render;
use render::{
    generate_crate_pages, generate_pages, generate_user_pages, render_news_pages,
    render_static_pages,
};

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(
        long,
        default_value_t = 0,
        help = "Limit the number of items we process."
    )]
    limit: i32,

    #[arg(
        long,
        default_value_t = 0,
        help = "Number of git repositories to try to clone or pull."
    )]
    pull: u32,

    #[arg(
        long,
        default_value_t = 0,
        help = "Number of git repositories to process."
    )]
    vcs: u32,

    #[arg(long, default_value_t = false, help = "Generate HTML pages")]
    html: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Repo {
    display: String,
    name: String,
    url: String,
    count: usize,
    percentage: String,
    crates: Vec<Crate>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Crate {
    created_at: String,
    description: String,
    documentation: String,
    downloads: String,
    homepage: String,
    id: String,
    max_upload_size: String,
    name: String,
    readme: String,
    repository: String,
    updated_at: String,

    #[serde(default = "empty_string")]
    owner_gh_login: String,

    #[serde(default = "empty_string")]
    owner_name: String,

    #[serde(default = "empty_string")]
    owner_gh_avatar: String,

    #[serde(default = "empty_details")]
    details: Details,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
    gh_avatar: String,
    gh_id: String,
    gh_login: String,
    id: String,
    name: String,

    #[serde(default = "get_zero")]
    count: u16,
}

fn empty_details() -> Details {
    Details::new()
}

fn empty_string() -> String {
    "".to_string()
}

fn get_zero() -> u16 {
    0
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Team {
    avatar: String,
    github_id: String,
    login: String,
    id: String,
    name: String,
    org_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CrateOwner {
    crate_id: String,
    created_at: String,
    created_by: String,
    owner_id: String,
    owner_kind: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
struct Details {
    has_github_action: bool,
    has_gitlab_pipeline: bool,
    commit_count: i32,
    cargo_fix: String,
}

impl Details {
    pub fn new() -> Details {
        Details {
            has_github_action: false,
            has_gitlab_pipeline: false,
            commit_count: 0,
            cargo_fix: "".to_string(),
        }
    }
}

//type RepoPercentage<'a> = HashMap<&'a str, String>;
type Owners = HashMap<String, String>;
type CratesByOwner = HashMap<String, Vec<String>>;
// type Users = HashMap<String, User>;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    simple_logger::init_with_level(log::Level::Info).unwrap();
    let start_time = std::time::Instant::now();
    log::info!("Starting the Rust Digger");
    log::info!("{VERSION}");

    //    log::info!("Limit {args.limit}");

    let (owner_by_crate_id, crates_by_owner): (Owners, CratesByOwner) =
        read_crate_owners(args.limit);
    let mut users = read_users(args.limit);
    read_teams(&mut users, args.limit);
    let mut crates: Vec<Crate> = read_crates(args.limit);
    //dbg!(&crates_by_owner);

    add_owners_to_crates(&mut crates, &users, &owner_by_crate_id);

    update_repositories(&crates, args.pull);
    collect_data_from_vcs(&mut crates, args.vcs);

    if args.html {
        generate_pages(&crates)?;
        render_news_pages();
        render_static_pages()?;
        generate_crate_pages(&crates)?;
        generate_user_pages(&crates, users, &crates_by_owner)?;
    }

    log::info!("Elapsed time: {} sec.", start_time.elapsed().as_secs());
    log::info!("Ending the Rust Digger");
    Ok(())
}

fn collect_data_from_vcs(crates: &mut Vec<Crate>, vcs: u32) {
    log::info!("process VCS");

    let mut count: u32 = 0;
    for krate in crates {
        if vcs <= count {
            break;
        }
        let mut details = Details {
            has_github_action: false,
            has_gitlab_pipeline: false,
            commit_count: 0,
            cargo_fix: "".to_string(),
        };
        log::info!(
            "process ({}/{}) repository '{}'",
            count,
            vcs,
            &krate.repository
        );
        if krate.repository == "" {
            continue;
        }
        let (host, owner, repo) = get_owner_and_repo(&krate.repository);
        if owner == "" {
            continue;
        }
        let repo_path = format!("repos/{host}/{owner}/{repo}");
        if !Path::new(&repo_path).exists() {
            log::warn!("Cloned path does not exist for {}", &krate.repository);
            continue;
        }
        let current_dir = env::current_dir().unwrap();
        env::set_current_dir(&repo_path).unwrap();

        if host == "github" {
            let workflows = Path::new(".github/workflows");
            if workflows.exists() {
                for entry in workflows.read_dir().expect("read_dir call failed") {
                    if let Ok(entry) = entry {
                        log::info!("workflows: {:?}", entry.path());
                        details.has_github_action = true;
                    }
                }
            }
        }
        if host == "gitlab" {
            let gitlab_ci_file = Path::new(".gitlab-ci.yml");
            details.has_gitlab_pipeline = gitlab_ci_file.exists();
        }
        if host != "" {
            details.commit_count = git_get_count();
        }
        build_docker_image();
        run_cargo_in_docker();
        details.cargo_fix = git_status();
        git_checkout();

        krate.details = details;
        env::set_current_dir(&current_dir).unwrap();
        count += 1;
    }
}

/// docker build -t rust-test .
fn build_docker_image() {
    let result = Command::new("docker")
        .arg("build")
        .arg("-t")
        .arg("rust-test")
        .arg(".")
        .output()
        .expect("Could not run");
    log::info!("build_docker_image {:?}", result.status.code());
}

/// docker run --rm --workdir /opt -v$(pwd):/opt -it --user tester rust-test cargo fix
fn run_cargo_in_docker() {
    let result = Command::new("docker")
        .arg("run")
        .arg("--rm")
        .arg("--workdir")
        .arg("/opt")
        .arg("-it")
        .arg("--user")
        .arg("tester")
        .arg("rust-test")
        .arg("cargo")
        .arg("fix")
        .output()
        .expect("Could not run");
    log::info!("run_cargo_in_docker {:?}", result.status.code());
}

//git status --porcelain
fn git_status() -> String {
    let result = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .output()
        .expect("Could not run");
    log::info!("build_docker_image {:?}", result.status.code());
    let stdout = std::str::from_utf8(&result.stdout).unwrap();
    stdout.to_string()
}

fn git_checkout() {
    let result = Command::new("git")
        .arg("checkout")
        .arg(".")
        .output()
        .expect("Could not run");
    log::info!("git checkout {:?}", result.status.code());
}

fn update_repositories(crates: &Vec<Crate>, pull: u32) {
    log::info!("start update repositories");

    let _res = fs::create_dir_all("repos");
    let _res = fs::create_dir_all("repos/github");
    let _res = fs::create_dir_all("repos/gitlab");
    let mut repo_reuse: HashMap<String, i32> = HashMap::new();

    let mut count: u32 = 0;
    for krate in crates {
        if pull <= count {
            break;
        }
        if krate.repository == "" {
            continue;
        }

        let repository = krate.repository.to_lowercase();
        *repo_reuse.entry(repository.clone()).or_insert(0) += 1;
        if *repo_reuse.get(&repository as &str).unwrap() > 1 {
            continue;
        }

        let (host, owner, repo) = get_owner_and_repo(&repository);
        if owner == "" {
            continue;
        }

        log::info!(
            "update ({}/{}) repository '{}'",
            count,
            pull,
            krate.repository
        );
        let owner_path = format!("repos/{host}/{owner}");
        let _res = fs::create_dir_all(&owner_path);
        let repo_path = format!("{owner_path}/{repo}");
        let current_dir = env::current_dir().unwrap();
        if Path::new(&repo_path).exists() {
            env::set_current_dir(&repo_path).unwrap();
            git_pull();
        } else {
            env::set_current_dir(owner_path).unwrap();
            git_clone(&krate.repository, &repo);
        }

        env::set_current_dir(current_dir).unwrap();
        count += 1;
    }
}

fn git_get_count() -> i32 {
    let result = Command::new("git")
        .arg("rev-list")
        .arg("HEAD")
        .arg("--count")
        .output()
        .expect("Could not run");

    if result.status.success() {
        let stdout = std::str::from_utf8(&result.stdout).unwrap().trim_end();
        //log::info!("'{}'", stdout);
        let number: i32 = stdout.parse().unwrap();
        number
    } else {
        0
    }
}

fn git_clone(url: &str, path: &str) {
    log::info!("git clone {} {}", url, path);
    let result = Command::new("git")
        .arg("clone")
        .arg(url)
        .arg(path)
        .output()
        .expect("Could not run");
    if result.status.success() {
        log::info!("git_clone exit code {}", result.status);
    } else {
        log::warn!("git_clone exit code {}", result.status);
    }
}

fn git_pull() {
    log::info!("git pull");
    let result = Command::new("git")
        .arg("pull")
        .output()
        .expect("Could not run");
    if result.status.success() {
        log::info!("git_pull exit code {}", result.status);
    } else {
        log::warn!("git_pull exit code {}", result.status);
    }
}

fn add_owners_to_crates(crates: &mut Vec<Crate>, users: &Vec<User>, owner_by_crate_id: &Owners) {
    let mut mapping: HashMap<String, &User> = HashMap::new();
    for user in users {
        mapping.insert(user.id.clone(), user);
    }

    for krate in crates.into_iter() {
        let crate_id = &krate.id;
        match owner_by_crate_id.get(crate_id) {
            Some(owner_id) => {
                //println!("owner_id: {owner_id}");
                match mapping.get(owner_id) {
                    Some(val) => {
                        krate.owner_gh_login = val.gh_login.clone();
                        krate.owner_name = val.name.clone();
                        krate.owner_gh_avatar = val.gh_avatar.clone();
                    }
                    None => {
                        log::warn!("crate {crate_id} owner_id {owner_id} does not have a user");
                    }
                }
            }
            None => {
                log::warn!("crate {crate_id} does not have an owner");
            }
        }
    }
}

fn get_repo_types() -> Vec<Repo> {
    let repos: Vec<Repo> = vec![
        Repo {
            display: "GitHub".to_string(),
            name: "github".to_string(),
            url: "https://github.com/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "GitLab".to_string(),
            name: "gitlab".to_string(),
            url: "https://gitlab.com/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Codeberg".to_string(),
            name: "codeberg".to_string(),
            url: "https://codeberg.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Gitee".to_string(),
            name: "gitee".to_string(),
            url: "https://gitee.com/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Tor Project (GitLab)".to_string(),
            name: "torproject".to_string(),
            url: "https://gitlab.torproject.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Free Desktop (GitLab)".to_string(),
            name: "freedesktop".to_string(),
            url: "https://gitlab.freedesktop.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "CERN (GitLab)".to_string(),
            name: "cern".to_string(),
            url: "https://gitlab.cern.ch/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Wikimedia (GitLab)".to_string(),
            name: "wikimedia".to_string(),
            url: "https://gitlab.wikimedia.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "e3t".to_string(),
            name: "e3t".to_string(),
            url: "https://git.e3t.cc/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "srht".to_string(),
            name: "srht".to_string(),
            url: "https://git.sr.ht/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Open Privacy".to_string(),
            name: "openprivacy".to_string(),
            url: "https://git.openprivacy.ca/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Cronce (GitLab)".to_string(),
            name: "cronce".to_string(),
            url: "https://gitlab.cronce.io/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Gnome (GitLab)".to_string(),
            name: "gnome".to_string(),
            url: "https://gitlab.gnome.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "Repo with http".to_string(),
            name: "repo-with-http".to_string(),
            url: "http://".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
        Repo {
            display: "GitHub with www".to_string(),
            name: "github-with-www".to_string(),
            url: "https://www.github.com/".to_string(),
            count: 0,
            percentage: "0".to_string(),
            crates: vec![],
        },
    ];
    repos
}

fn collect_repos(crates: &Vec<Crate>) -> Vec<Repo> {
    log::info!("collect_repos start");
    let mut repos: Vec<Repo> = get_repo_types();
    let mut no_repo: Vec<Crate> = vec![];
    let mut other_repo: Vec<Crate> = vec![];

    for krate in crates {
        if krate.repository == "" {
            no_repo.push(krate.clone());
            continue;
        }
        let mut matched = false;
        repos = repos
            .into_iter()
            .map(|mut repo| {
                if krate.repository.starts_with(&repo.url) {
                    repo.count += 1;
                    matched = true;
                    repo.crates.push(krate.clone());
                }
                repo
            })
            .collect();

        if !matched {
            other_repo.push(krate.clone());
        }
    }

    repos.push(Repo {
        display: "Has no repository".to_string(),
        name: "no-repo".to_string(),
        url: "".to_string(),
        count: no_repo.len(),
        percentage: "0".to_string(),
        crates: no_repo,
    });

    repos.push(Repo {
        display: "Other repositories we don't recognize".to_string(),
        name: "other-repos".to_string(),
        url: "".to_string(),
        count: other_repo.len(),
        percentage: "0".to_string(),
        crates: other_repo,
    });

    repos = repos
        .into_iter()
        .map(|mut repo| {
            repo.percentage = percentage(repo.count, crates.len());
            repo
        })
        .collect();

    log::info!("collect_repos end");
    repos
}

#[cfg(test)]
mod tests {

    // #[test]
    // fn test_has_repo() {
    //     let x = Crate {
    //         created_at: "".to_string(),
    //         description: "".to_string(),
    //         documentation: "".to_string(),
    //         downloads: "".to_string(),
    //         homepage: "".to_string(),
    //         id: "".to_string(),
    //         max_upload_size: "".to_string(),
    //         name: "".to_string(),
    //         readme: "".to_string(),
    //         repository: "https://github.com/szabgab/rust-digger".to_string(),
    //         updated_at: "".to_string(),
    //     };
    //     assert!(has_repo(&x));

    //     let x = Crate {
    //         created_at: "".to_string(),
    //         description: "".to_string(),
    //         documentation: "".to_string(),
    //         downloads: "".to_string(),
    //         homepage: "".to_string(),
    //         id: "".to_string(),
    //         max_upload_size: "".to_string(),
    //         name: "".to_string(),
    //         readme: "".to_string(),
    //         repository: "".to_string(),
    //         updated_at: "".to_string(),
    //     };
    //     assert!(!has_repo(&x));
    // }
}
