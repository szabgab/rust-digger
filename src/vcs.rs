use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write as _;
use std::path::Path;
use std::process::Command;

use clap::Parser;
use toml::Table;

use git_digger::Repository;

use rust_digger::{
    collected_data_root, get_repos_folder, load_vcs_details, read_crates, save_details, Crate,
    ElapsedTimer, VCSDetails,
};

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
    simple_logger::init_with_level(log::Level::Info).unwrap();
    log::info!("Starting the VCS processor.");
    let start_time = std::time::Instant::now();

    match run() {
        Ok(()) => {}
        Err(err) => log::error!("Error: {err}"),
    }

    log::info!("Elapsed time: {} sec.", start_time.elapsed().as_secs());
    log::info!("Ending the VCS processor");
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    log::info!("Limit: {}", args.limit);
    let _a = ElapsedTimer::new("vcs.rs");

    let crates: Vec<Crate> = read_crates(0)?;
    collect_data_from_vcs(&crates, args.limit)?;

    Ok(())
}

fn collect_data_from_vcs(
    crates: &Vec<Crate>,
    limit: u32,
) -> Result<(), Box<dyn std::error::Error>> {
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

        let repository = match Repository::from_url(&krate.repository) {
            Ok(repo) => repo,
            Err(err) => {
                log::error!("Error: {err} when parsing the repository url");
                continue;
            }
        };

        if seen.contains(&krate.repository.to_lowercase()) {
            continue;
        }
        //log::info!("{:?}", seen);
        seen.insert(krate.repository.to_lowercase());

        let mut details = load_vcs_details(&krate.repository);

        let repo_path = repository.path(&get_repos_folder());
        if !Path::new(&repo_path).exists() {
            log::warn!("Cloned path does not exist for {}", &krate.repository);
            continue;
        }
        let current_dir = env::current_dir()?;
        env::set_current_dir(&repo_path)?;
        log::info!("in folder: {:?}", env::current_dir()?);

        collect_data_about_ci(&mut details)?;

        collect_data_about_rustfmt(&mut details, &mut rustfmt, krate);

        if !repository.host.is_empty() {
            details.commit_count = git_get_count();
        }

        env::set_current_dir(&current_dir)?;
        save_details(&krate.repository, &details)?;

        count += 1;
    }

    save_rustfm(&rustfmt);

    Ok(())
}

fn collect_data_about_rustfmt(details: &mut VCSDetails, rustfmt: &mut Vec<String>, krate: &Crate) {
    details.has_rustfmt_toml = Path::new("rustfmt.toml").exists();
    details.has_dot_rustfmt_toml = Path::new(".rustfmt.toml").exists();
    if details.has_rustfmt_toml {
        read_rustfmt(rustfmt, "rustfmt.toml", &krate.name);
    }
    if details.has_dot_rustfmt_toml {
        read_rustfmt(rustfmt, ".rustfmt.toml", &krate.name);
    }
}

fn collect_data_about_ci(details: &mut VCSDetails) -> Result<(), Box<dyn Error>> {
    let workflows = Path::new(".github/workflows");
    if workflows.exists() {
        for entry in workflows.read_dir()?.flatten() {
            log::info!("workflows: {:?}", entry.path());
            details.has_github_action = true;
        }
    }

    details.has_gitlab_pipeline = Path::new(".gitlab-ci.yml").exists();
    details.has_circle_ci = Path::new(".circleci").exists();
    details.has_cirrus_ci = Path::new(".cirrus.yaml").exists();
    details.has_travis_ci = Path::new(".travis.yaml").exists();
    details.has_jenkins = Path::new("Jenkinsfile").exists();
    details.has_appveyor =
        Path::new(".appveyor.yml").exists() || Path::new("appveyor.yml").exists();
    details.has_azure_pipeline = Path::new("azure-pipelines.yml").exists();
    details.has_bitbucket_pipeline = Path::new("bitbucket-pipelines.yml").exists();

    Ok(())
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
                        #[expect(clippy::string_slice)]
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
