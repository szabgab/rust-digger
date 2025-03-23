use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

use clap::Parser;

use chrono::{DateTime, Duration, NaiveDateTime, Utc};

use git_digger::get_owner_and_repo;

use rust_digger::{get_repos_folder, load_vcs_details, read_crates, Crate, ElapsedTimer};

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

    #[arg(long, default_value_t = false, help = "Only clone, don't pull.")]
    clone: bool,

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
    simple_logger::init_with_level(log::Level::Info).unwrap();
    let start_time = std::time::Instant::now();

    match run() {
        Ok(()) => {}
        Err(err) => log::error!("Error: {err}"),
    }

    log::info!("Elapsed time: {} sec.", start_time.elapsed().as_secs());
    log::info!("Ending the clone process");
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    let _a = ElapsedTimer::new("clone.rs");
    log::info!("Starting the clone process for max {} crates.", args.limit);

    let crates: Vec<Crate> = read_crates(0)?;
    update_repositories(&crates, args.limit, args.recent, args.force, args.clone)?;

    Ok(())
}

fn update_repositories(
    crates: &Vec<Crate>,
    limit: u32,
    recent: u32,
    force: bool,
    clone: bool,
) -> Result<(), Box<dyn Error>> {
    log::info!("start update repositories");

    std::env::set_var("GIT_TERMINAL_PROMPT", "0");

    let mut repo_reuse: HashMap<String, i32> = HashMap::new(); // number of times each repository is used for crates (monorepo)
    let now: DateTime<Utc> = Utc::now();
    let before: DateTime<Utc> = now
        - Duration::try_days(recent as i64)
            .ok_or_else(|| Box::<dyn Error>::from("Could not convert recent"))?;
    log::info!("before: {}", before);

    let mut count: u32 = 0;
    for krate in crates {
        if 0 < limit && limit <= count {
            break;
        }
        //log::info!("update_at {}", krate.updated_at); // 2023-09-18 01:44:10.299066
        log::info!("Crate {} updated_at: {}", krate.name, krate.updated_at);
        if 0 < recent && crate_too_old(krate, before) {
            continue;
        }

        let repository_url = get_repository_url(krate);

        if repository_url.is_empty() {
            continue;
        }

        match repo_reuse.get(&repository_url as &str) {
            Some(value) => {
                repo_reuse.insert(repository_url.clone(), value + 1);
                continue;
            }
            None => repo_reuse.insert(repository_url.clone(), 1),
        };

        let (host, owner, repo) = get_owner_and_repo(&repository_url);
        if owner.is_empty() {
            continue;
        }

        let details = load_vcs_details(&repository_url);
        if !details.git_clone_error.is_empty() && !force {
            continue;
        }

        log::info!("update ({count}/{limit}) repository '{}'", &repository_url);

        let status = check_url(&repository_url);
        if status != 200 {
            log::error!(
                "Error accessing the repository '{}' status: {}",
                &repository_url,
                status
            );
            continue;
        }

        update_single_repository(&host, &owner, &repo, &repository_url, clone)?;

        count += 1;
    }

    Ok(())
}

fn get_repository_url(krate: &Crate) -> String {
    if !krate.repository.is_empty() {
        return krate.repository.to_lowercase();
    }

    if !krate.homepage.is_empty() {
        log::info!(
            "Trying to use homepage field '{}' as a repository link to clone the project",
            krate.homepage
        );
        return krate.homepage.to_lowercase();
    }

    String::new()
}

fn update_single_repository(
    host: &str,
    owner: &str,
    repo: &str,
    repository_url: &str,
    clone: bool,
) -> Result<(), Box<dyn Error>> {
    let owner_path = get_repos_folder().join(host).join(owner);
    let current_dir = env::current_dir()?;
    log::info!(
        "Creating owner_path {:?} while current_dir is {:?}",
        &owner_path,
        &current_dir
    );
    fs::create_dir_all(&owner_path)?;
    let repo_path = owner_path.join(repo);
    if Path::new(&repo_path).exists() {
        if clone {
            log::info!("repo exist but we only clone now.  Skipping.");
        } else {
            log::info!("repo exist; cd to {:?}", &repo_path);
            env::set_current_dir(&repo_path)?;
            git_pull();
        }
    } else {
        log::info!("new repo; cd to {:?}", &owner_path);
        env::set_current_dir(owner_path)?;
        git_clone(repository_url, repo);
    }
    env::set_current_dir(current_dir)?;
    Ok(())
}

fn crate_too_old(krate: &Crate, before: DateTime<Utc>) -> bool {
    let updated_at = match NaiveDateTime::parse_from_str(&krate.updated_at, "%Y-%m-%d %H:%M:%S.%f")
    {
        Ok(ts) => ts,
        Err(err) => {
            // TODO there are some crates, eg. one called cargo-script where the
            // updated_at field has no microseconds and it looks like this: 2023-09-18 01:44:10
            log::error!(
                "Error parsing timestamp '{}' of the crate {} ({})",
                &krate.updated_at,
                &krate.name,
                err
            );
            //std::process::exit(1);
            return true;
        }
    };
    if updated_at < before.naive_utc() {
        return true;
    }

    false
}

fn git_clone(url: &str, path: &str) {
    log::info!("git clone {} {}", url, path);
    match Command::new("git").arg("clone").arg(url).arg(path).output() {
        Ok(result) => {
            if result.status.success() {
                log::info!("git_clone exit code: '{}'", result.status);
            } else {
                log::warn!(
                    "git_clone exit code: '{}' for url '{}' cloning to '{}'",
                    result.status,
                    url,
                    path
                );
            }
        }
        Err(err) => log::error!("Could not run git_clone {url} {path} error: {err}"),
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
    let current_dir = env::current_dir().unwrap();

    match Command::new("git").arg("pull").output() {
        Ok(result) => {
            if result.status.success() {
                log::info!(
                    "git_pull exit code: '{}' in folder {:?}",
                    result.status,
                    current_dir
                );
            } else {
                log::warn!(
                    "git_pull exit code: '{}' in folder {:?}",
                    result.status,
                    current_dir
                );
            }
        }
        Err(err) => log::error!("Could not run git_pull in folder {current_dir:?} error: {err}"),
    }
}
