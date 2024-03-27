use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use clap::Parser;

use chrono::{DateTime, Duration, NaiveDateTime, Utc};

mod macros;
use macros::return_or_exit;

use rust_digger::{get_owner_and_repo, load_details, read_crates, Crate};

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(
        long,
        default_value_t = 0,
        help = "Limit the number of repos we process."
    )]
    limit: u32,

    #[arg(
        long,
        default_value_t = 0,
        help = "Attempt to clone only repos of crates that were released in the last `recent` days."
    )]
    recent: u32,

    #[arg(
        long,
        default_value_t = false,
        help = "Try to clone even if it already failed once."
    )]
    force: bool,
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
    let start_time = std::time::Instant::now();

    log::info!("Starting the clone process {}", args.limit);

    let crates: Vec<Crate> = return_or_exit!(read_crates(0), 2);
    update_repositories(&crates, args.limit, args.recent, args.force);
    log::info!("Elapsed time: {} sec.", start_time.elapsed().as_secs());
    log::info!("Ending the clone process");
}

fn update_repositories(crates: &Vec<Crate>, limit: u32, recent: u32, force: bool) {
    log::info!("start update repositories");

    let mut repo_reuse: HashMap<String, i32> = HashMap::new(); // number of times each repository is used for crates (monorepo)
    let now: DateTime<Utc> = Utc::now();
    let before: DateTime<Utc> = now - Duration::try_days(recent as i64).unwrap();
    log::info!("before: {}", before);

    let mut count: u32 = 0;
    for krate in crates {
        if 0 < limit && limit <= count {
            break;
        }
        //log::info!("update_at {}", krate.updated_at); // 2023-09-18 01:44:10.299066
        log::info!("Crate {} updated_at: {}", krate.name, krate.updated_at);
        if 0 < recent {
            let updated_at =
                match NaiveDateTime::parse_from_str(&krate.updated_at, "%Y-%m-%d %H:%M:%S.%f") {
                    Ok(ts) => ts,
                    Err(err) => {
                        // TODO there are some crates, eg. one called cargo-script where the
                        // updated_at field has no microseconds and it looks like this: 2023-09-18 01:44:10
                        log::error!(
                            "Error parsing timestamp '{}' of {} ({})",
                            &krate.updated_at,
                            &krate.name,
                            err
                        );
                        //std::process::exit(1);
                        continue;
                    }
                };
            if updated_at < before.naive_utc() {
                continue;
            }
        }

        if krate.repository.is_empty() {
            continue;
        }

        let repository = krate.repository.to_lowercase();
        *repo_reuse.entry(repository.clone()).or_insert(0) += 1;
        if *repo_reuse.get(&repository as &str).unwrap() > 1 {
            continue;
        }

        let (host, owner, repo) = get_owner_and_repo(&repository);
        if owner.is_empty() {
            continue;
        }

        let details = load_details(&repository);
        if !details.git_clone_error.is_empty() && !force {
            continue;
        }

        log::info!(
            "update ({}/{}) repository '{}'",
            count,
            limit,
            krate.repository
        );
        let owner_path = format!("repos/{host}/{owner}");
        let current_dir = env::current_dir().unwrap();
        log::info!(
            "Creating owner_path '{}' while current_dir is {:?}",
            &owner_path,
            &current_dir
        );
        fs::create_dir_all(&owner_path).unwrap();
        let repo_path = format!("{owner_path}/{repo}");
        let status = check_url(&krate.repository);
        if status != 200 {
            log::error!(
                "Error accessing the repository {}. status: {}",
                &krate.repository,
                status
            );
            continue;
        }
        if Path::new(&repo_path).exists() {
            log::info!("repo exist; cd to {}", &repo_path);
            env::set_current_dir(&repo_path).unwrap();
            git_pull();
        } else {
            log::info!("new repo; cd to {}", &owner_path);
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

fn check_url(url: &str) -> reqwest::StatusCode {
    log::info!("Checking url {}", url);

    let res = match reqwest::blocking::get(url) {
        Ok(res) => res,
        Err(err) => {
            log::error!("Could not get '{}': {}", url, err);
            return reqwest::StatusCode::INTERNAL_SERVER_ERROR;
        }
    };
    log::info!("Status: {}", res.status());
    res.status()
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
