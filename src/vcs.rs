use std::collections::HashSet;
use std::env;
use std::path::Path;
use std::process::Command;

use clap::Parser;

use rust_digger::{
    get_owner_and_repo, get_repos_folder, load_details, read_crates, save_details, Crate,
};

mod macros;
use macros::ok_or_exit;

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

fn main() {
    let args = Cli::parse();
    simple_logger::init_with_level(log::Level::Info).unwrap();
    log::info!("Starting the VCS processor {}", args.limit);

    let crates: Vec<Crate> = ok_or_exit!(read_crates(0), 3);
    collect_data_from_vcs(&crates, args.limit);

    log::info!("Ending the VCS processor");
}

fn collect_data_from_vcs(crates: &Vec<Crate>, limit: u32) {
    log::info!("process collect_data_from_vcs start");
    log::info!("Total number of crates: {}", crates.len());
    if 0 < limit {
        log::info!("We are going to process only {} crates", limit);
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut count: u32 = 0;
    for krate in crates {
        if 0 < limit && limit <= count {
            break;
        }
        log::info!(
            "process ({}/{}) repository '{}'",
            count,
            limit,
            &krate.repository
        );
        if krate.repository.is_empty() {
            continue;
        }

        let (host, owner, repo) = get_owner_and_repo(&krate.repository);
        if owner.is_empty() {
            continue;
        }

        if seen.contains(&krate.repository.to_lowercase()) {
            continue;
        }
        //log::info!("{:?}", seen);
        seen.insert(krate.repository.to_lowercase());

        let mut details = load_details(&krate.repository);

        let repo_path = get_repos_folder().join(&host).join(&owner).join(&repo);
        if !Path::new(&repo_path).exists() {
            log::warn!("Cloned path does not exist for {}", &krate.repository);
            continue;
        }
        let current_dir = env::current_dir().unwrap();
        env::set_current_dir(&repo_path).unwrap();
        log::info!("in folder: {:?}", env::current_dir().unwrap());

        if host == "github" {
            details.has_github_action = false;
            let workflows = Path::new(".github/workflows");
            if workflows.exists() {
                for entry in workflows
                    .read_dir()
                    .expect("read_dir call failed")
                    .flatten()
                {
                    log::info!("workflows: {:?}", entry.path());
                    details.has_github_action = true;
                }
            }
        }
        if host == "gitlab" {
            let gitlab_ci_file = Path::new(".gitlab-ci.yml");
            details.has_gitlab_pipeline = gitlab_ci_file.exists();
        }
        details.cargo_toml_in_root = Path::new("Cargo.toml").exists();
        details.has_rustfmt_toml = Path::new("rustfmt.toml").exists();
        details.has_dot_rustfmt_toml = Path::new(".rustfmt.toml").exists();

        if !host.is_empty() {
            details.commit_count = git_get_count();
        }

        env::set_current_dir(&current_dir).unwrap();
        save_details(&krate.repository, &details);

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
        let stdout = core::str::from_utf8(&result.stdout).unwrap().trim_end();
        //log::info!("'{}'", stdout);
        let number: i32 = stdout.parse().unwrap();
        number
    } else {
        0
    }
}
