use std::collections::HashMap;
use std::env;
use std::error::Error;

use clap::Parser;

mod macros;
use macros::return_or_exit;

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
    simple_logger::init_with_level(log::Level::Info)?;

    let start_time = std::time::Instant::now();
    log::info!("Starting the Rust Digger");
    log::info!("{VERSION}");
    //log::info!("Limit {args.limit}");

    // load crates information from CSV files
    let (owner_by_crate_id, crates_by_owner): (Owners, CratesByOwner) =
        read_crate_owners(args.limit)?;
    let mut users = read_users(args.limit)?;
    read_teams(&mut users, args.limit)?;
    let mut crates: Vec<Crate> = return_or_exit!(read_crates(args.limit), 1);

    //dbg!(&crates_by_owner);

    add_owners_to_crates(&mut crates, &users, &owner_by_crate_id);
    load_details_for_all_the_crates(&mut crates);

    let repos = collect_repos(&crates);

    std::thread::scope(|scope| {
        scope.spawn(|| generate_pages(&crates, &repos).unwrap());
        scope.spawn(render_news_pages);
        scope.spawn(|| render_static_pages().unwrap());
        scope.spawn(|| generate_crate_pages(&crates).unwrap());
        scope.spawn(|| generate_user_pages(&crates, users, &crates_by_owner).unwrap());
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
    let text = include_str!("../repo_types.yaml");

    let repos: Vec<Repo> = serde_yaml::from_str(text).unwrap();
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
        display: String::from("Has no repository"),
        name: String::from("no-repo"),
        url: String::new(),
        count: no_repo.len(),
        percentage: String::from("0"),
        crates: no_repo,
        platform: None,
        bold: true,
    });

    repos.push(Repo {
        display: String::from("Other repositories we don't recognize"),
        name: String::from("other-repos"),
        url: String::new(),
        count: other_repo.len(),
        percentage: String::from("0"),
        crates: other_repo,
        platform: None,
        bold: true,
    });

    repos = repos
        .into_iter()
        .map(|mut repo| {
            repo.percentage = percentage(repo.count, crates.len());
            repo
        })
        .collect();

    repos.sort_unstable_by(|repoa, repob| {
        (repob.count, repob.name.to_lowercase()).cmp(&(repoa.count, repoa.name.to_lowercase()))
    });

    log::info!("collect_repos end");
    repos
}

#[test]
fn test_get_repo_types() {
    let _repos = get_repo_types();
}
