use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use chrono::prelude::{DateTime, Utc};
use clap::Parser;
use regex::Regex;

pub type Partials = liquid::partials::EagerCompiler<liquid::partials::InMemorySource>;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PAGE_SIZE: usize = 100;

mod read;
use read::{read_crate_owners, read_crates, read_teams, read_users};
mod render;
use render::{
    generate_crate_pages, generate_user_pages, load_templates, render_list_crates_by_repo,
    render_list_of_repos, render_list_page, render_news_pages, render_static_pages,
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
}

impl Details {
    pub fn new() -> Details {
        Details {
            has_github_action: false,
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
        };
        log::info!(
            "process ({}/{}) repository '{}'",
            count,
            vcs,
            &krate.repository
        );
        let (owner, repo) = get_owner_and_repo(&krate.repository);
        if owner == "" {
            continue;
        }
        let repo_path = format!("repos/github/{owner}/{repo}");
        if !Path::new(&repo_path).exists() {
            log::error!("Cloned path does not exist for {}", &krate.repository);
            continue;
        }
        let current_dir = env::current_dir().unwrap();
        env::set_current_dir(&repo_path).unwrap();

        let workflows = Path::new(".github/workflows");
        if workflows.exists() {
            for entry in workflows.read_dir().expect("read_dir call failed") {
                if let Ok(entry) = entry {
                    log::info!("workflows: {:?}", entry.path());
                    details.has_github_action = true;
                }
            }
        }
        krate.details = details;
        env::set_current_dir(&current_dir).unwrap();
        count += 1;
    }
}

fn get_owner_and_repo(repository: &str) -> (String, String) {
    let re = Regex::new(r"^https://github.com/([^/]+)/([^/]+)$").unwrap();
    let repo_url = match re.captures(&repository) {
        Some(value) => value,
        None => {
            println!("No match");
            return ("".to_string(), "".to_string());
        }
    };
    let owner = repo_url[1].to_lowercase();
    let repo = repo_url[2].to_lowercase();
    (owner, repo)
}

fn update_repositories(crates: &Vec<Crate>, pull: u32) {
    log::info!("start update repositories");

    let _res = fs::create_dir_all("repos");
    let _res = fs::create_dir_all("repos/github");

    let mut count: u32 = 0;
    for krate in crates {
        if pull <= count {
            break;
        }
        let (owner, repo) = get_owner_and_repo(&krate.repository);
        if owner == "" {
            continue;
        }

        log::info!(
            "update ({}/{}) repository '{}'",
            count,
            pull,
            krate.repository
        );
        let owner_path = format!("repos/github/{owner}");
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

fn git_clone(url: &str, path: &str) {
    log::info!("git clone {} {}", url, path);
    let result = Command::new("git")
        .arg("clone")
        .arg(url)
        .arg(path)
        .output()
        .expect("Could not run");
    log::info!("Run command exit code {}", result.status);
}

fn git_pull() {
    log::info!("git pull");
    let result = Command::new("git")
        .arg("pull")
        .output()
        .expect("Could not run");
    log::info!("Run command exit code {}", result.status);
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

// fn has_repo(w: &Crate) -> bool {
//     w.repository != ""
// }
fn has_homepage_no_repo(w: &Crate) -> bool {
    w.homepage != "" && w.repository == ""
}
fn no_homepage_no_repo(w: &Crate) -> bool {
    w.homepage == "" && w.repository == ""
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

    repos
}

fn percentage(num: usize, total: usize) -> String {
    let t = (10000 * num / total) as f32;
    (t / 100.0).to_string()
}

fn generate_pages(crates: &Vec<Crate>) -> Result<(), Box<dyn Error>> {
    log::info!("generate_pages");

    // Create a folder _site
    let _res = fs::create_dir_all("_site");
    let _res = fs::create_dir_all("_site/crates");
    let _res = fs::create_dir_all("_site/users");
    let _res = fs::create_dir_all("_site/news");
    let _res = fs::create_dir_all("_site/vcs");

    fs::copy("digger.js", "_site/digger.js")?;

    let all_crates: Vec<Crate> = crates.into_iter().cloned().collect();
    let home_page_but_no_repo = crates
        .into_iter()
        .filter(|w| has_homepage_no_repo(w))
        .cloned()
        .collect::<Vec<Crate>>();
    let no_homepage_no_repo_crates = crates
        .into_iter()
        .filter(|w| no_homepage_no_repo(w))
        .cloned()
        .collect::<Vec<Crate>>();

    let crates_without_owner_name = crates
        .into_iter()
        .filter(|krate| krate.owner_name == "")
        .cloned()
        .collect::<Vec<Crate>>();

    let crates_without_owner = crates
        .into_iter()
        .filter(|krate| krate.owner_name == "" && krate.owner_gh_login == "")
        .cloned()
        .collect::<Vec<Crate>>();

    let repos = collect_repos(&crates);

    render_list_crates_by_repo(&repos)?;
    render_list_of_repos(&repos);

    render_list_page(
        &"_site/index.html".to_string(),
        &"Rust Digger".to_string(),
        &all_crates,
    )?;

    render_list_page(
        &"_site/has-homepage-but-no-repo.html".to_string(),
        &"Has homepage, but no repository".to_string(),
        &home_page_but_no_repo,
    )?;

    render_list_page(
        &"_site/no-homepage-no-repo.html".to_string(),
        &"No repository, no homepage".to_string(),
        &no_homepage_no_repo_crates,
    )?;

    render_list_page(
        &"_site/crates-without-owner-name.html".to_string(),
        &"Crates without owner name".to_string(),
        &crates_without_owner_name,
    )?;

    render_list_page(
        &"_site/crates-without-owner.html".to_string(),
        &"Crates without owner".to_string(),
        &crates_without_owner,
    )?;

    //log::info!("repos: {:?}", repos);

    log::info!("render_stats_page");
    let partials = match load_templates() {
        Ok(partials) => partials,
        Err(error) => panic!("Error loading templates {}", error),
    };

    let template = liquid::ParserBuilder::with_stdlib()
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/stats.html")
        .unwrap();

    let filename = "_site/stats.html";
    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   "Rust Digger Stats",
        //"user":    user,
        //"crate":   krate,
        "total": crates.len(),
        "repos": repos,
        "home_page_but_no_repo": home_page_but_no_repo.len(),
        "home_page_but_no_repo_percentage":  percentage(home_page_but_no_repo.len(), crates.len()),
        "no_homepage_no_repo_crates": no_homepage_no_repo_crates.len(),
        "no_homepage_no_repo_crates_percentage": percentage(no_homepage_no_repo_crates.len(), crates.len()),
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{}", html).unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_percentage() {
        assert_eq!(percentage(20, 100), "20");
        assert_eq!(percentage(5, 20), "25");
        assert_eq!(percentage(1234, 10000), "12.34");
        assert_eq!(percentage(1234567, 10000000), "12.34");
    }
}
