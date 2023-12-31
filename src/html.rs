use std::collections::HashMap;
use std::env;
use std::error::Error;

use clap::Parser;

pub type Partials = liquid::partials::EagerCompiler<liquid::partials::InMemorySource>;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PAGE_SIZE: usize = 100;

use rust_digger::{
    load_details, percentage, read_crates, Crate, CratesByOwner, Owners, Repo, User,
};
mod read;
use read::{read_crate_owners, read_teams, read_users};
mod render;
use render::{
    generate_crate_pages, generate_pages, generate_robots_txt, generate_sitemap,
    generate_user_pages, render_news_pages, render_static_pages,
};

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(
        long,
        default_value_t = 0,
        help = "Limit the number of items we process."
    )]
    limit: u32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let start_time = std::time::Instant::now();
    log::info!("Starting the Rust Digger");
    log::info!("{VERSION}");
    //log::info!("Limit {args.limit}");

    // load crates information from CSV files
    let (owner_by_crate_id, crates_by_owner): (Owners, CratesByOwner) =
        read_crate_owners(args.limit);
    let mut users = read_users(args.limit);
    read_teams(&mut users, args.limit);
    let mut crates: Vec<Crate> = read_crates(args.limit);
    //dbg!(&crates_by_owner);

    add_owners_to_crates(&mut crates, &users, &owner_by_crate_id);
    load_details_for_all_the_crates(&mut crates);

    let repos = collect_repos(&crates);

    std::thread::scope(|s| {
        s.spawn(|| generate_pages(&crates, &repos).unwrap());
        s.spawn(render_news_pages);
        s.spawn(|| render_static_pages().unwrap());
        s.spawn(|| generate_crate_pages(&crates).unwrap());
        s.spawn(|| generate_user_pages(&crates, users, &crates_by_owner).unwrap());
    });

    generate_sitemap();
    generate_robots_txt();

    log::info!("Elapsed time: {} sec.", start_time.elapsed().as_secs());
    log::info!("Ending the Rust Digger generating html pages");
    Ok(())
}

// fn save_repo_details(crates: &Vec<Crate>) {
//     log::info!("start saving details");

//     let _res = fs::create_dir_all("repos");
//     let _res = fs::create_dir_all("repos/github");
//     let _res = fs::create_dir_all("repos/gitlab");

//     for krate in crates {
//         if krate.repository == "" {
//             continue;
//         }

//         let repository = krate.repository.to_lowercase();
//         *repo_reuse.entry(repository.clone()).or_insert(0) += 1;
//         if *repo_reuse.get(&repository as &str).unwrap() > 1 {
//             continue;
//         }

//         let (host, owner, repo) = get_owner_and_repo(&repository);
//         if owner == "" {
//             continue;
//         }

//         log::info!(
//             "update ({}/{}) repository '{}'",
//             count,
//             pull,
//             krate.repository
//         );
//         let owner_path = format!("repos/{host}/{owner}");
//         let _res = fs::create_dir_all(&owner_path);
//     }
// }

fn load_details_for_all_the_crates(crates: &mut [Crate]) {
    for krate in crates.iter_mut() {
        krate.details = load_details(&krate.repository);
    }
}

fn add_owners_to_crates(crates: &mut [Crate], users: &Vec<User>, owner_by_crate_id: &Owners) {
    let mut mapping: HashMap<String, &User> = HashMap::new();
    for user in users {
        mapping.insert(user.id.clone(), user);
    }

    for krate in crates.iter_mut() {
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
        if krate.repository.is_empty() {
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
