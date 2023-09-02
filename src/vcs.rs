use std::env;
use std::path::Path;
use std::process::Command;

use clap::Parser;

mod read;
use read::read_crates;

mod common;
use common::{get_owner_and_repo, save_details, Crate, Details};

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

    let crates: Vec<Crate> = read_crates(0);
    collect_data_from_vcs(&crates, args.limit);

    log::info!("Starting the VCS processor {}", args.limit);
}

fn collect_data_from_vcs(crates: &Vec<Crate>, limit: u32) {
    log::info!("process collect_data_from_vcs start");
    log::info!("Total number of crates: {}", crates.len());
    if 0 < limit {
        log::info!("We are going to process only {} crates", limit);
    }

    let mut count: u32 = 0;
    for krate in crates {
        // TODO: avoid processing the same repo twice in the same run, or shall we update the info listing both crates?
        if 0 < limit && limit <= count {
            break;
        }
        let mut details = Details::new();
        // TODO load details of already exist

        log::info!(
            "process ({}/{}) repository '{}'",
            count,
            limit,
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
        details.cargo_toml_in_root = Path::new("Cargo.toml").exists();

        if host != "" {
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
        let stdout = std::str::from_utf8(&result.stdout).unwrap().trim_end();
        //log::info!("'{}'", stdout);
        let number: i32 = stdout.parse().unwrap();
        number
    } else {
        0
    }
}
