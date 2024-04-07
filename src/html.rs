use std::collections::HashMap;
use std::env;
use std::error::Error;

use clap::Parser;

mod macros;
use macros::ok_or_exit;

pub type Partials = liquid::partials::EagerCompiler<liquid::partials::InMemorySource>;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PAGE_SIZE: usize = 100;

use rust_digger::{
    collected_data_root, load_details, read_crates, Crate, CratesByOwner, Owners, Repo, User,
};
mod read;
use read::{read_crate_owners, read_teams, read_users};
mod render;
use render::{
    create_folders, generate_crate_pages, generate_pages, generate_robots_txt, generate_sitemap,
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
    let (owner_by_crate_id, crates_by_owner): (Owners, CratesByOwner) = read_crate_owners()?;
    let mut users = read_users(args.limit)?;
    read_teams(&mut users, args.limit)?;
    let mut crates: Vec<Crate> = ok_or_exit!(read_crates(args.limit), 1);

    //dbg!(&crates_by_owner);

    add_owners_to_crates(&mut crates, &users, &owner_by_crate_id);
    load_details_for_all_the_crates(&mut crates);
    create_folders();

    std::thread::scope(|scope| {
        scope.spawn(|| generate_pages(&crates).unwrap());
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

//     let _res = fs::create_dir_all("repos"); // get_repos_folder()
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
