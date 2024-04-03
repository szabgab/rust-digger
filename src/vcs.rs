use std::collections::HashSet;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use clap::Parser;
use toml::Table;

use rust_digger::{
    collected_data_root, get_owner_and_repo, get_repos_folder, load_details, read_crates,
    save_details, Crate,
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

    let mut rustfmt: Vec<String> = vec![];
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
        if details.has_rustfmt_toml {
            read_rustfmt(&mut rustfmt, "rustfmt.toml", &krate.name);
        }
        if details.has_dot_rustfmt_toml {
            read_rustfmt(&mut rustfmt, ".rustfmt.toml", &krate.name);
        }

        if !host.is_empty() {
            details.commit_count = git_get_count();
        }

        env::set_current_dir(&current_dir).unwrap();
        save_details(&krate.repository, &details);

        count += 1;
    }

    save_rustfm(&rustfmt);
}

fn save_rustfm(rustfmt: &[String]) {
    fs::create_dir_all(collected_data_root()).unwrap();
    let filename = collected_data_root().join("rustfmt.txt");
    let mut file = File::create(filename).unwrap();
    for entry in rustfmt {
        writeln!(&mut file, "{entry}").unwrap();
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

fn read_rustfmt(rustfmt: &mut Vec<String>, filename: &str, name: &str) {
    match std::fs::read_to_string(filename) {
        Err(err) => {
            log::error!("Error: {err} when reading {filename} of {name}");
        }
        Ok(content) => {
            match content.parse::<Table>() {
                Err(err) => {
                    log::error!("Error: {err} when parsing toml in {filename} of {name}");
                }
                Ok(table) => {
                    for row in &table {
                        //log::debug!("key: {:30} value: {}", row.0, row.1);
                        let mut value = row.1.to_string();
                        if value.starts_with('"') && value.ends_with('"') {
                            value = value[1..value.len() - 1].to_owned();
                        }

                        rustfmt.push(format!("{},{},{}", row.0, value, name));
                    }
                }
            }
        }
    }
}
