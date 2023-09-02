fn main() {

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

