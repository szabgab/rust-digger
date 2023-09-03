use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use clap::Parser;

mod read;
use read::read_crates;

mod common;
use common::{get_owner_and_repo, Crate};

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(
        long,
        default_value_t = 0,
        help = "Limit the number of repos we process."
    )]
    limit: u32,
}

/// for each crate
///     get the url and type of the VCS
///     load the details of vcs
///
///     if there is no clone yet:
///         if we have evidence that the cloning has already failed then got to next
///         else try to clone
///             if failed
///                 save in the details
///             else
///                 collect data from repo and save that in the details
///
///     if there is already a clone
///         if the crate was released recently then run git pull
///             if there are new commits
///                  collect data from repo and save that in the details
///
///     (if the data collection takes too long we might need to separate it from the cloning)
fn main() {
    let args = Cli::parse();
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let crates: Vec<Crate> = read_crates(0);
    update_repositories(&crates, args.limit);

    log::info!("Starting the VCS processor {}", args.limit);
}

fn update_repositories(crates: &Vec<Crate>, limit: u32) {
    log::info!("start update repositories");

    let mut repo_reuse: HashMap<String, i32> = HashMap::new();

    let mut count: u32 = 0;
    for krate in crates {
        if 0 < limit && limit <= count {
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
            limit,
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
