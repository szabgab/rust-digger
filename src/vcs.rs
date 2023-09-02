use std::fs;
use std::path::Path;
use std::process::Command;

use clap::Parser;

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

    log::info!("Starting the VCS processor {}", args.limit);
}

//collect_data_from_vcs(&mut crates, args.vcs);

// fn collect_data_from_vcs(crates: &mut Vec<Crate>, vcs: u32) {
//     log::info!("process VCS");

//     let mut count: u32 = 0;
//     for krate in crates {
//         if vcs <= count {
//             break;
//         }
//         let mut details = Details::new();
//         log::info!(
//             "process ({}/{}) repository '{}'",
//             count,
//             vcs,
//             &krate.repository
//         );
//         if krate.repository == "" {
//             continue;
//         }
//         let (host, owner, repo) = get_owner_and_repo(&krate.repository);
//         if owner == "" {
//             continue;
//         }
//         let repo_path = format!("repos/{host}/{owner}/{repo}");
//         if !Path::new(&repo_path).exists() {
//             log::warn!("Cloned path does not exist for {}", &krate.repository);
//             continue;
//         }
//         let current_dir = env::current_dir().unwrap();
//         env::set_current_dir(&repo_path).unwrap();

//         if host == "github" {
//             let workflows = Path::new(".github/workflows");
//             if workflows.exists() {
//                 for entry in workflows.read_dir().expect("read_dir call failed") {
//                     if let Ok(entry) = entry {
//                         log::info!("workflows: {:?}", entry.path());
//                         details.has_github_action = true;
//                     }
//                 }
//             }
//         }
//         if host == "gitlab" {
//             let gitlab_ci_file = Path::new(".gitlab-ci.yml");
//             details.has_gitlab_pipeline = gitlab_ci_file.exists();
//         }
//         details.cargo_toml_in_root = Path::new("Cargo.toml").exists();

//         if host != "" {
//             details.commit_count = git_get_count();
//         }

//         krate.details = details;
//         env::set_current_dir(&current_dir).unwrap();
//         count += 1;
//     }
// }

// fn update_repositories(crates: &Vec<Crate>, pull: u32) {
//     log::info!("start update repositories");

//     let _res = fs::create_dir_all("repo-details");
//     let _res = fs::create_dir_all("repos-details/github");
//     let _res = fs::create_dir_all("repos-details/gitlab");

//     let mut repo_reuse: HashMap<String, i32> = HashMap::new();

//     let mut count: u32 = 0;
//     for krate in crates {
//         if pull <= count {
//             break;
//         }
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
//         let repo_path = format!("{owner_path}/{repo}");
//         let current_dir = env::current_dir().unwrap();
//         if Path::new(&repo_path).exists() {
//             env::set_current_dir(&repo_path).unwrap();
//             git_pull();
//         } else {
//             env::set_current_dir(owner_path).unwrap();
//             git_clone(&krate.repository, &repo);
//         }

//         env::set_current_dir(current_dir).unwrap();
//         count += 1;
//     }
// }

// fn git_get_count() -> i32 {
//     let result = Command::new("git")
//         .arg("rev-list")
//         .arg("HEAD")
//         .arg("--count")
//         .output()
//         .expect("Could not run");

//     if result.status.success() {
//         let stdout = std::str::from_utf8(&result.stdout).unwrap().trim_end();
//         //log::info!("'{}'", stdout);
//         let number: i32 = stdout.parse().unwrap();
//         number
//     } else {
//         0
//     }
// }

// fn git_clone(url: &str, path: &str) {
//     log::info!("git clone {} {}", url, path);
//     let result = Command::new("git")
//         .arg("clone")
//         .arg(url)
//         .arg(path)
//         .output()
//         .expect("Could not run");
//     if result.status.success() {
//         log::info!("git_clone exit code {}", result.status);
//     } else {
//         log::warn!("git_clone exit code {}", result.status);
//     }
// }

// fn git_pull() {
//     log::info!("git pull");
//     let result = Command::new("git")
//         .arg("pull")
//         .output()
//         .expect("Could not run");
//     if result.status.success() {
//         log::info!("git_pull exit code {}", result.status);
//     } else {
//         log::warn!("git_pull exit code {}", result.status);
//     }
// }
