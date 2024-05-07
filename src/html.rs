use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::prelude::{DateTime, Utc};
use clap::Parser;
use liquid_filter_commafy::Commafy;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use thousands::Separable;

use rust_digger::{
    add_cargo_toml_to_crates, analyzed_crates_root, build_path, collected_data_root,
    get_owner_and_repo, load_crate_details, load_vcs_details, percentage, read_crates,
    CargoTomlErrors, Crate, CrateErrors, CratesByOwner, Owners, Repo, User,
};

const URL: &str = "https://rust-digger.code-maven.com";

pub type Partials = liquid::partials::EagerCompiler<liquid::partials::InMemorySource>;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PAGE_SIZE: usize = 200;

struct CrateFilter {
    func: Box<dyn Fn(&&Crate) -> bool>,
}

impl CrateFilter {
    fn new<F>(func: F) -> Self
    where
        F: Fn(&&Crate) -> bool + 'static,
    {
        Self {
            func: Box::new(func),
        }
    }
}

#[derive(Serialize, Debug)]
struct StatEntry<'aaa> {
    path: &'aaa str,
    title: &'aaa str,
    count: usize,
    percentage: String,
}

mod read;
use read::{read_crate_owners, read_teams, read_users};

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
    let (mut crates, released_cargo_toml_errors, released_cargo_toml_errors_nameless) =
        add_cargo_toml_to_crates(read_crates(args.limit)?)?;

    //dbg!(&crates_by_owner);

    add_owners_to_crates(&mut crates, &users, &owner_by_crate_id);
    load_vcs_details_for_all_the_crates(&mut crates);
    load_crate_details_for_all_the_crates(&mut crates);
    create_html_folders()?;

    std::thread::scope(|scope| {
        scope.spawn(|| generate_pages(&crates, &released_cargo_toml_errors).unwrap());
        scope.spawn(|| generate_interesting_homepages(&crates).unwrap());
        scope.spawn(|| generate_errors_page(&released_cargo_toml_errors_nameless).unwrap());
        scope.spawn(render_news_pages);
        scope.spawn(|| render_static_pages().unwrap());
        scope.spawn(|| generate_crate_pages(&crates, &released_cargo_toml_errors).unwrap());
        scope.spawn(|| {
            generate_user_pages(
                &crates,
                users,
                &crates_by_owner,
                &released_cargo_toml_errors,
            )
            .unwrap();
        });
    });

    generate_top_crates_lists(&mut crates).unwrap();

    generate_sitemap();
    generate_robots_txt();

    log::info!("Elapsed time: {} sec.", start_time.elapsed().as_secs());
    log::info!("Ending the Rust Digger generating html pages");
    Ok(())
}

fn load_vcs_details_for_all_the_crates(crates: &mut [Crate]) {
    for krate in crates.iter_mut() {
        krate.vcs_details = load_vcs_details(&krate.repository);
    }
}

fn load_crate_details_for_all_the_crates(crates: &mut [Crate]) {
    #[allow(clippy::pattern_type_mismatch)]
    for krate in crates.iter_mut() {
        let filename = format!(
            "{}-{}.json",
            krate.cargo.package.name, krate.cargo.package.version
        );
        let filepath = analyzed_crates_root().join(filename);
        krate.crate_details = load_crate_details(&filepath).unwrap_or_default();
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
                        krate.owner_gh_login.clone_from(&val.gh_login);
                        krate.owner_name.clone_from(&val.name);
                        krate.owner_gh_avatar.clone_from(&val.gh_avatar);
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

fn get_site_folder() -> PathBuf {
    PathBuf::from("_site")
}

pub fn render_list_of_repos(repos: &Vec<Repo>) {
    log::info!("render_list_of_repos start");
    let partials = load_templates().unwrap();

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/repos.html")
        .unwrap();

    let filename = get_site_folder().join("vcs").join("index.html");
    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   String::from("Repositories"),
        "repos":    repos,
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{html}").unwrap();
    log::info!("render_list_of_repos end");
}

pub fn load_templates() -> Result<Partials, Box<dyn Error>> {
    // log::info!("load_templates");

    let mut partials = Partials::empty();
    for filename in [
        "templates/incl/header.html",
        "templates/incl/footer.html",
        "templates/incl/navigation.html",
        "templates/incl/list_crates.html",
        "templates/incl/list_crate_errors.html",
    ] {
        partials.add(filename, fs::read_to_string(filename).unwrap());
    }

    Ok(partials)
}

pub fn render_static_pages() -> Result<(), Box<dyn Error>> {
    log::info!("render_static_pages start");

    let pages = vec![
        ("index", "Rust Digger"),
        ("about-ci", "About Continuous Integration for Rust"),
        (
            "about-repository",
            "About Public Version Control for Rust projects",
        ),
        ("about-fmt", "About cargo fmt"),
        ("about-cargo-toml", "About Cargo.toml"),
        ("about", "About Rust Digger"),
        ("support", "Support Rust Digger"),
        ("training", "Training courses"),
    ];

    for page in pages {
        let partials = load_templates().unwrap();

        let utc: DateTime<Utc> = Utc::now();
        let globals = liquid::object!({
            "version": format!("{VERSION}"),
            "utc":     format!("{}", utc),
            "title":   page.1,
        });

        let template = liquid::ParserBuilder::with_stdlib()
            .filter(Commafy)
            .partials(partials)
            .build()
            .unwrap()
            .parse_file(format!("templates/{}.html", page.0))
            .unwrap();
        let html = template.render(&globals).unwrap();

        let mut file =
            File::create(build_path(get_site_folder(), &[page.0], Some("html"))).unwrap();
        writeln!(&mut file, "{html}").unwrap();
    }
    log::info!("render_static_pages end");
    Ok(())
}

pub fn render_list_page(
    filename: &str,
    title: &str,
    crates: &[&Crate],
) -> Result<(), Box<dyn Error>> {
    log::info!("render_list_page: {filename:?}");

    let filepath = std::path::PathBuf::from(format!(
        "{}.html",
        get_site_folder().join(filename).display()
    ));
    log::info!("render_file: {filepath:?}");

    let partials = load_templates().unwrap();

    let page_size = if crates.len() > PAGE_SIZE {
        PAGE_SIZE
    } else {
        crates.len()
    };

    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   title,
        "filename": filename,
        "total":   crates.len(),
        "crates":  (crates[0..page_size]).to_vec(),
    });

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/crate_list_page.html")
        .unwrap();
    let html = template.render(&globals).unwrap();

    let mut file = File::create(filepath).unwrap();
    writeln!(&mut file, "{html}").unwrap();
    //match res {
    //    Ok(html) => writeln!(&mut file, "{}", html).unwrap(),
    //    Err(error) => log:error!("{}", error)
    //}
    Ok(())
}

pub fn render_news_pages() {
    log::info!("render_news_pages");
    let utc: DateTime<Utc> = Utc::now();

    let news_path = Path::new("templates/news");
    let Ok(dir_handle) = news_path.read_dir() else {
        log::error!("Could not read directory {:?}", news_path);
        return;
    };

    for entry in dir_handle.flatten() {
        let partials = load_templates().unwrap();
        let path = entry.path();
        let Some(extension) = path.extension() else {
            log::warn!("file without extension: {:?}", &path);
            continue;
        };

        if extension != std::ffi::OsStr::new("html") {
            log::warn!("file with invalid extension: {:?} (not html)", &path);
            continue;
        }

        log::info!("news file: {:?}", path);
        log::info!("{:?}", path.strip_prefix("templates/"));
        let output_path =
            get_site_folder().join(path.strip_prefix("templates/").unwrap().as_os_str());
        let template = liquid::ParserBuilder::with_stdlib()
            .filter(Commafy)
            .partials(partials)
            .build()
            .unwrap()
            .parse_file(path)
            .unwrap();

        let globals = liquid::object!({
            "version": format!("{VERSION}"),
            "utc":     format!("{}", utc),
        });
        let html = template.render(&globals).unwrap();
        //let filename = "_site/news.html";
        let mut file = File::create(output_path).unwrap();
        writeln!(&mut file, "{html}").unwrap();
    }
}

pub fn generate_crate_pages(
    crates: &Vec<Crate>,
    released_cargo_toml_errors: &CrateErrors,
) -> Result<(), Box<dyn Error>> {
    log::info!("generate_crate_pages start");
    let partials = load_templates().unwrap();

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/crate.html")
        .unwrap();

    for krate in crates {
        let filename = build_path(get_site_folder(), &["crates", &krate.name], Some("html"));
        let utc: DateTime<Utc> = Utc::now();
        //log::info!("{:?}", krate);
        //std::process::exit(1);

        let def = String::new();
        let cargo_toml_error = released_cargo_toml_errors.get(&krate.name).unwrap_or(&def);

        let globals = liquid::object!({
            "version": format!("{VERSION}"),
            "utc":     format!("{}", utc),
            "title":   &krate.name,
            "crate":   krate,
            "cargo_toml_error": cargo_toml_error,
        });
        let html = template.render(&globals).unwrap();
        let mut file = File::create(filename).unwrap();
        writeln!(&mut file, "{html}").unwrap();
    }
    log::info!("generate_crate_pages end");
    Ok(())
}

pub fn generate_user_pages(
    crates: &Vec<Crate>,
    users: Vec<User>,
    crates_by_owner: &CratesByOwner,
    released_cargo_toml_errors: &CrateErrors,
) -> Result<(), Box<dyn Error>> {
    log::info!("generate_user_pages start");

    let partials = load_templates().unwrap();

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/user.html")
        .unwrap();

    let mut crate_by_id: HashMap<&str, &Crate> = HashMap::new();
    for krate in crates {
        crate_by_id.insert(&krate.id, krate);
    }
    //dbg!(&crate_by_id);
    //dbg!(&crate_by_id["81366"]);

    let mut users_with_crates: Vec<User> = users
        .into_iter()
        .map(|mut user| {
            let mut selected_crates: Vec<&Crate> = vec![];
            if let Some(crate_ids) = crates_by_owner.get(&user.id) {
                //dbg!(crate_ids);
                for crate_id in crate_ids {
                    //log::debug!("crate_id: {}", &crate_id);
                    //log::info!("crate_by_id: {:#?}", crate_by_id);
                    //log::info!("crate_by_id: {:#?}", crate_by_id.keys());
                    //dbg!(&crate_id);
                    //dbg!(&crate_by_id[crate_id.as_str()]);
                    //dbg!(&crate_by_id.get(&crate_id.clone()));
                    if crate_by_id.contains_key(crate_id.as_str()) {
                        selected_crates.push(crate_by_id[crate_id.as_str()]);
                    }
                }
                user.count = selected_crates.len();
                //users_with_crates.push(user);

                #[allow(clippy::min_ident_chars)]
                selected_crates.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                let filename = build_path(
                    get_site_folder(),
                    &["users", &user.gh_login.to_ascii_lowercase()],
                    Some("html"),
                );

                let mut problems: HashMap<&str, Vec<&&Crate>> = HashMap::new();
                problems.insert(
                    "vcs_with_http",
                    selected_crates
                        .iter()
                        .filter(|krate| {
                            !krate.repository.is_empty() && krate.repository.starts_with("http://")
                        })
                        .collect::<Vec<_>>(),
                );

                problems.insert(
                    "vcs_with_www",
                    selected_crates
                        .iter()
                        .filter(|krate| {
                            !krate.repository.is_empty()
                                && krate.repository.starts_with("https://www.github.com")
                        })
                        .collect::<Vec<_>>(),
                );

                problems.insert(
                    "both_rustfm_and_dot_rustfmt",
                    selected_crates
                        .iter()
                        .filter(|krate| {
                            krate.vcs_details.has_rustfmt_toml
                                && krate.vcs_details.has_dot_rustfmt_toml
                        })
                        .collect::<Vec<_>>(),
                );

                problems.insert(
                    "has_cargo_toml_errors",
                    selected_crates
                        .iter()
                        .filter(|krate| released_cargo_toml_errors.contains_key(&krate.name))
                        .collect::<Vec<_>>(),
                );

                let utc: DateTime<Utc> = Utc::now();
                let globals = liquid::object!({
                    "version": format!("{VERSION}"),
                    "utc":     format!("{}", utc),
                    "title":   &user.name,
                    "user":    user,
                    "crates":  selected_crates,
                    "problems": problems,
                });
                let html = template.render(&globals).unwrap();
                let mut file = File::create(filename).unwrap();
                writeln!(&mut file, "{html}").unwrap();
            }
            user
        })
        .filter(|user| user.count > 0)
        .collect();

    #[allow(clippy::min_ident_chars)]
    users_with_crates.sort_by(|a, b| a.name.cmp(&b.name));

    generate_list_of_users(&users_with_crates);
    save_list_of_users_json(&users_with_crates);
    generate_people_search_page();

    log::info!("generate_user_pages end");
    Ok(())
}

fn save_list_of_users_json(users: &[User]) {
    let mut users_data = users
        .iter()
        .map(|user| {
            HashMap::from([
                ("name", user.name.clone()),
                ("gh_login", user.gh_login.to_lowercase()),
            ])
        })
        .collect::<Vec<HashMap<&str, String>>>();
    #[allow(clippy::min_ident_chars)]
    users_data.sort_by(|a, b| a["gh_login"].cmp(&b["gh_login"]));

    match serde_json::to_string(&users_data) {
        Err(err) => log::error!("Could not serialize user list {err}"),
        Ok(data) => {
            let filename = get_site_folder().join("users.json");
            let mut file = File::create(filename).unwrap();
            writeln!(&mut file, "{data}").unwrap();
        }
    }
}

fn generate_list_of_users(users: &Vec<User>) {
    log::info!("generate_list_of_users start");
    // list all the users on the /users/ page
    let partials = load_templates().unwrap();

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/users.html")
        .unwrap();

    let filename = get_site_folder().join("users").join("index.html");
    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   String::from("Users"),
        "users":    users,
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{html}").unwrap();
    log::info!("generate_list_of_users end");
}

fn generate_people_search_page() {
    log::info!("generate_people_search_page start");

    let partials = load_templates().unwrap();

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/people.html")
        .unwrap();

    let filename = get_site_folder().join("people.html");
    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   String::from("People"),
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{html}").unwrap();

    log::info!("generate_people_search_page end");
}

fn render_stats_page(stats: &[StatEntry]) {
    log::info!("render_stats_page");
    let partials = load_templates().unwrap();

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/stats.html")
        .unwrap();

    let filename = get_site_folder().join("stats.html");
    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   "Rust Digger Stats",
        "stats": stats,
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{html}").unwrap();
}

fn create_html_folders() -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(get_site_folder())?;
    for folder in ["crates", "users", "news", "vcs", "rustfmt"] {
        fs::create_dir_all(get_site_folder().join(folder))?;
    }

    Ok(())
}

fn collect_paths(root: &Path) -> Vec<String> {
    log::info!("collect_paths  from {:?}", root);

    let mut paths: Vec<String> = vec![];
    for entry in root.read_dir().expect("failed") {
        //log::info!("{}", &format!("{}", entry.unwrap().path().display())[5..])
        //paths.push(format!("{}", entry.unwrap().path().display())[5..].to_string().clone());
        let path = entry.as_ref().unwrap().path();
        #[allow(clippy::string_slice)]
        if path.is_file() && path.extension().unwrap() == "html" {
            let url_path =
                format!("{}", path.display())[5..path.display().to_string().len() - 5].to_string();
            if url_path.ends_with("/index") {
                paths.push(url_path[0..url_path.len() - 5].to_string());
            } else {
                paths.push(url_path);
            }
        }
        if path.is_dir() {
            let basename = path.file_name().unwrap();
            if basename == "crates" || basename == "users" {
                continue;
            }
            paths.extend(collect_paths(path.as_path()));
        }
    }
    paths
}
pub fn generate_sitemap() {
    log::info!("generate_sitemap");
    let paths = collect_paths(&get_site_folder());
    //log::info!("{:?}", paths);

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .build()
        .unwrap()
        .parse_file("templates/sitemap.xml")
        .unwrap();

    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "url": URL,
        "timestamp":  utc.format("%Y-%m-%d").to_string(),
        "pages":    paths,
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(get_site_folder().join("sitemap.xml")).unwrap();
    writeln!(&mut file, "{html}").unwrap();
}

pub fn generate_robots_txt() {
    let text = format!("Sitemap: {URL}/sitemap.xml\n\nUser-agent: *\n");
    let mut file = File::create(get_site_folder().join("robots.txt")).unwrap();
    writeln!(&mut file, "{text}").unwrap();
}

fn collect_repos(crates: &[Crate]) -> Result<usize, Box<dyn Error>> {
    log::info!("collect_repos start");
    let mut repos: Vec<Repo> = get_repo_types();

    let no_repo_count = render_filtered_crates(
        "vcs/no-repo",
        "Crates without repository", // Crates in Has no repository
        |krate| krate.repository.is_empty(),
        crates,
    )?;

    let other_repo_count = render_filtered_crates(
        "vcs/other-repos",
        "Crates with other repositories we don't recognize",
        |krate| {
            !(krate.repository.is_empty()
                || repos
                    .iter()
                    .any(|repo| krate.repository.starts_with(&repo.url)))
        },
        crates,
    )?;

    repos = repos
        .into_iter()
        .map(|mut repo| {
            let count = render_filtered_crates(
                &format!("vcs/{}", &repo.name),
                &format!("Crates in {}", repo.display),
                |krate| krate.repository.starts_with(&repo.url),
                crates,
            )
            .unwrap();

            repo.count = count;
            repo.percentage = percentage(repo.count, crates.len());

            repo
        })
        .collect();

    repos.push(Repo {
        display: String::from("Other repositories we don't recognize"),
        name: String::from("other-repos"),
        url: String::new(),
        count: other_repo_count,
        percentage: percentage(other_repo_count, crates.len()),
        platform: None,
        bold: true,
    });

    repos.push(Repo {
        display: String::from("Has no repository"),
        name: String::from("no-repo"),
        url: String::new(),
        count: no_repo_count,
        percentage: percentage(no_repo_count, crates.len()),
        platform: None,
        bold: true,
    });

    repos.sort_unstable_by(|repoa, repob| {
        (repob.count, repob.name.to_lowercase()).cmp(&(repoa.count, repoa.name.to_lowercase()))
    });

    render_list_of_repos(&repos);

    log::info!("collect_repos end");
    Ok(no_repo_count)
}

pub fn generate_errors_page(
    released_cargo_toml_errors_nameless: &CargoTomlErrors,
) -> Result<(), Box<dyn Error>> {
    log::info!("generate_errors_page");
    let partials = load_templates().unwrap();

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/errors.html")
        .unwrap();

    let filename = get_site_folder().join("errors.html");
    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   "Errors",
        "released_cargo_toml_errors_nameless": released_cargo_toml_errors_nameless,
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{html}").unwrap();

    Ok(())
}

fn crate_has_interesting_homepage(krate: &Crate) -> bool {
    krate
        .cargo
        .package
        .homepage
        .as_ref()
        .map_or(false, |homepage| {
            !homepage.starts_with("https://github.com/")
                && !homepage.starts_with("http://github.com/")
                && !homepage.starts_with("https://gitlab.com/")
                && !homepage.starts_with("https://crates.io/")
                && !homepage.starts_with("https://docs.rs/")
                && !homepage.starts_with("https://libs.rs/")
        })
}

pub fn generate_interesting_homepages(crates: &[Crate]) -> Result<(), Box<dyn Error>> {
    log::info!("generate_interesting_homepages");

    let homepages = crates.iter().filter_map(|krate| {
        if crate_has_interesting_homepage(krate) {
            krate.cargo.package.homepage.clone()
        } else {
            None
        }
    });

    let mut seen: HashSet<String> = HashSet::new();

    let mut unique_homepages = homepages
        .into_iter()
        .filter(|hp| {
            if seen.contains(hp) {
                false
            } else {
                seen.insert(hp.clone());
                true
            }
        })
        .collect::<Vec<String>>();
    unique_homepages.sort();

    log::info!(
        "generate_interesting_homepages results: {} {:#?}",
        unique_homepages.len(),
        unique_homepages
    );

    let partials = load_templates().unwrap();

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/homepages.html")
        .unwrap();

    let filename = get_site_folder().join("homepages.html");
    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   "homepages",
        "homepages": unique_homepages,
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{html}").unwrap();

    Ok(())
}

fn render_top_crates(
    filename: &str,
    title: &str,
    fields: &[&str],
    krates: &[Thing],
) -> Result<(), Box<dyn Error>> {
    log::info!("render_top_crates: {filename}",);

    let page_size = if krates.len() > PAGE_SIZE {
        PAGE_SIZE
    } else {
        krates.len()
    };

    let partials = load_templates().unwrap();

    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   title,
        "filename": filename,
        "total":   krates.len(),
        "fields":  fields,
        "things":  (krates[0..page_size]).to_vec(),
    });

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/list_top_crates.html")
        .unwrap();
    let html = template.render(&globals).unwrap();

    let filepath = std::path::PathBuf::from(format!(
        "{}.html",
        get_site_folder().join(filename).display()
    ));

    let mut file = File::create(filepath).unwrap();
    writeln!(&mut file, "{html}").unwrap();
    //match res {
    //    Ok(html) => writeln!(&mut file, "{}", html).unwrap(),
    //    Err(error) => log:error!("{}", error)
    //}
    Ok(())
}

#[derive(Debug, serde::Serialize, Clone)]
struct Thing<'local> {
    krate: &'local Crate,
    fields: Vec<String>,
}
pub fn generate_top_crates_lists(crates: &mut [Crate]) -> Result<(), Box<dyn Error>> {
    log::info!("start generate_top_crates_lsts");

    crates.sort_by_key(|krate| krate.crate_details.size);
    crates.reverse();

    let crates_and_fields = crates
        .iter()
        .map(|krate| Thing {
            krate,
            fields: vec![krate.crate_details.size.separate_with_commas()],
        })
        .collect::<Vec<Thing>>();

    render_top_crates(
        "biggest-crates",
        "Crates using the most bytes",
        &["Size"],
        &crates_and_fields,
    )?;

    log::info!("end generate_top_crates_lsts");
    Ok(())
}

/// Generate various lists of crates:
/// Filter the crates according to various rules and render them using `render_filtered_crates`.
/// Then using the numbers returned by that function generate the stats page.
#[allow(clippy::too_many_lines)]
pub fn generate_pages(
    crates: &[Crate],
    released_cargo_toml_errors: &CrateErrors,
) -> Result<(), Box<dyn Error>> {
    log::info!("generate_pages");

    fs::copy("digger.js", get_site_folder().join("digger.js"))?;

    let no_repo = collect_repos(crates)?;

    let _all = render_filtered_crates("all", "Rust Digger", |_krate| true, crates)?;

    let mut stats = vec![];

    stats.push(StatEntry {
        path: "all",
        title: "Total",
        count: crates.len(),
        percentage: String::from("100%"),
    });

    stats.push(StatEntry {
        path: "vcs/no-repo",
        title: "No repository",
        count: no_repo,
        percentage: percentage(no_repo, crates.len()),
    });

    let has_cargo_toml_errors = render_filtered_crates(
        "has-cargo-toml-errors",
        "Has errors in the released Cargo.toml file",
        |krate| released_cargo_toml_errors.contains_key(&krate.name),
        crates,
    )?;

    stats.push(StatEntry {
        path: "has-cargo-toml-errors",
        title: "Has errors in the released Cargo.toml file",
        count: has_cargo_toml_errors,
        percentage: percentage(has_cargo_toml_errors, crates.len()),
    });

    let cases = vec![
        (
            "github-but-no-ci",
            "On GitHub but has no CI",
            CrateFilter::new(|krate: &&Crate| on_github_but_no_ci(krate)),
        ),
        (
            "gitlab-but-no-ci",
            "On GitLab but has no CI",
            CrateFilter::new(|krate: &&Crate| on_gitlab_but_no_ci(krate)),
        ),
        (
            "has-cargo-toml-in-root",
            "Has Cargo.toml file in the root of the repository",
            CrateFilter::new(|krate: &&Crate| krate.vcs_details.cargo_toml_in_root),
        ),
        (
            "has-no-cargo-toml-in-root",
            "Has no Cargo.toml file in the root of the repository",
            CrateFilter::new(|krate: &&Crate| !krate.vcs_details.cargo_toml_in_root),
        ),
        (
            "has-homepage-but-no-repo",
            "Has homepage, but no repository",
            CrateFilter::new(|krate: &&Crate| {
                !krate.homepage.is_empty() && krate.repository.is_empty()
            }),
        ),
        (
            "has-rustfmt-toml",
            "Has rustfmt.toml file",
            CrateFilter::new(|krate: &&Crate| krate.vcs_details.has_rustfmt_toml),
        ),
        (
            "has-dot-rustfmt-toml",
            "Has .rustfmt.toml file",
            CrateFilter::new(|krate: &&Crate| krate.vcs_details.has_dot_rustfmt_toml),
        ),
        (
            "has-both-rustfmt-toml",
            "Has both rustfmt.toml and .rustfmt.toml file in the root of the repository",
            CrateFilter::new(|krate: &&Crate| {
                krate.vcs_details.has_rustfmt_toml && krate.vcs_details.has_dot_rustfmt_toml
            }),
        ),
        (
            "no-homepage-no-repo",
            "No repository, no homepage",
            CrateFilter::new(|krate: &&Crate| {
                krate.homepage.is_empty() && krate.repository.is_empty()
            }),
        ),
        (
            "crates-without-owner",
            "Crates without owner",
            CrateFilter::new(|krate: &&Crate| {
                krate.owner_name.is_empty() && krate.owner_gh_login.is_empty()
            }),
        ),
        (
            "crates-without-owner-name",
            "Crates without owner name",
            CrateFilter::new(|krate: &&Crate| krate.owner_name.is_empty()),
        ),
        (
            "crates-without-edition-or-rust-version",
            "Crates without edition or rust-version",
            CrateFilter::new(|krate: &&Crate| {
                krate.cargo.package.edition.is_none()
                    && krate.cargo.package.rust_dash_version.is_none()
            }),
        ),
        (
            "crates-with-both-edition-and-rust-version",
            "Crates with both edition and rust-version",
            CrateFilter::new(|krate: &&Crate| {
                krate.cargo.package.edition.is_some()
                    && krate.cargo.package.rust_dash_version.is_some()
            }),
        ),
        (
            "has-interesting-homepage",
            "Has interesting homepage",
            CrateFilter::new(|krate: &&Crate| crate_has_interesting_homepage(krate)),
        ),
        (
            "crates-with-cargo-lock",
            "Crates with Cargo.lock file",
            CrateFilter::new(|krate: &&Crate| krate.crate_details.has_cargo_lock),
        ),
        (
            "crates-without-cargo-lock",
            "Crates without Cargo.lock file",
            CrateFilter::new(|krate: &&Crate| !krate.crate_details.has_cargo_lock),
        ),
        (
            "crates-without-cargo-lock-without-main-rs",
            "Crates without Cargo.lock and without src/main.rs file",
            CrateFilter::new(|krate: &&Crate| {
                !krate.crate_details.has_cargo_lock && !krate.crate_details.has_main_rs
            }),
        ),
    ];
    for case in cases {
        let count = render_filtered_crates(case.0, case.1, case.2.func, crates)?;
        stats.push(StatEntry {
            path: case.0,
            title: case.1,
            count,
            percentage: percentage(count, crates.len()),
        });
    }

    render_stats_page(&stats);

    #[allow(clippy::if_then_some_else_none)]
    let with_rustfmt = stats
        .iter()
        .filter_map(|entry| {
            if entry.path == "has-dot-rustfmt-toml" || entry.path == "has-rustfmt-toml" {
                Some(entry.count)
            } else {
                None
            }
        })
        .sum();

    generate_rustfmt_pages(crates.len(), with_rustfmt, crates)?;
    generate_msrv_pages(crates)?;
    generate_ci_pages(crates)?;

    Ok(())
}

fn render_filtered_crates(
    filename: &str,
    title: &str,
    cond: impl Fn(&&Crate) -> bool,
    crates: &[Crate],
) -> Result<usize, Box<dyn Error>> {
    log::info!(
        "render_filtered_crates number of crates: {}, {filename}",
        crates.len()
    );
    let filtered_crates = crates.iter().filter(cond).collect::<Vec<&Crate>>();
    log::info!(
        "render_filtered_crates number of filtered crates: {}",
        filtered_crates.len()
    );
    render_list_page(filename, title, &filtered_crates)?;
    Ok(filtered_crates.len())
}

fn on_github_but_no_ci(krate: &Crate) -> bool {
    if krate.repository.is_empty() {
        return false;
    }

    let (host, owner, _) = get_owner_and_repo(&krate.repository);
    if owner.is_empty() {
        return false;
    }

    if host != "github" {
        return false;
    }

    if krate.vcs_details.has_github_action
        || krate.vcs_details.has_circle_ci
        || krate.vcs_details.has_cirrus_ci
    {
        return false;
    }

    true
}

fn on_gitlab_but_no_ci(krate: &Crate) -> bool {
    if krate.repository.is_empty() {
        return false;
    }

    let (host, owner, _) = get_owner_and_repo(&krate.repository);
    if owner.is_empty() {
        return false;
    }

    if host != "gitlab" {
        return false;
    }

    if krate.vcs_details.has_gitlab_pipeline {
        return false;
    }

    true
}

fn get_repo_types() -> Vec<Repo> {
    let text = include_str!("../repo_types.yaml");

    let repos: Vec<Repo> = serde_yaml::from_str(text).unwrap();
    repos
}

fn load_collected_rustfmt() -> Vec<(String, String, String)> {
    let mut rustfmt: Vec<(String, String, String)> = vec![];

    let filename = collected_data_root().join("rustfmt.txt");
    match std::fs::read_to_string(&filename) {
        Err(err) => {
            log::error!("Could not read {:?} {err}", filename);
        }
        Ok(content) => {
            for row in content.split('\n') {
                if row.is_empty() {
                    continue;
                }
                log::info!("{row}");
                let parts = row.split(',').collect::<Vec<&str>>();
                if parts.len() != 3 {
                    log::error!("Row '{row}' was split to {} parts", parts.len());
                    continue;
                }
                rustfmt.push((
                    parts[0].to_owned(),
                    parts[1].to_owned(),
                    parts[2].to_owned(),
                ));
            }
        }
    }

    rustfmt
}

fn generate_ci_pages(crates: &[Crate]) -> Result<(), Box<dyn Error>> {
    log::info!("generate_ci_pages start");

    let mut count: HashMap<&str, usize> = HashMap::new();

    count.insert(
        "has-github-actions",
        render_filtered_crates(
            "has-github-actions",
            "Crates with GitHub Actions",
            |krate| krate.vcs_details.has_github_action,
            crates,
        )?,
    );

    count.insert(
        "has-gitlab-pipeline",
        render_filtered_crates(
            "has-gitlab-pipeline",
            "Crates with GitLab Pipeline",
            |krate| krate.vcs_details.has_gitlab_pipeline,
            crates,
        )?,
    );

    count.insert(
        "has-cirrus-ci",
        render_filtered_crates(
            "has-cirrus-ci",
            "Crates with Cirrus CI",
            |krate| krate.vcs_details.has_cirrus_ci,
            crates,
        )?,
    );

    let partials = load_templates().unwrap();

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/ci.html")
        .unwrap();

    let filename = get_site_folder().join("ci.html");
    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   "CI systems",
        "count": count,
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{html}").unwrap();

    log::info!("generate_ci_pages end");
    Ok(())
}

fn vectorize(editions: &HashMap<String, u32>) -> Vec<(String, String, u32)> {
    let mut editions_vector = editions
        .iter()
        .map(|entry| {
            (
                entry.0.clone(),
                if entry.0.is_empty() {
                    String::from("na")
                } else {
                    entry.0.clone()
                },
                *entry.1,
            )
        })
        .collect::<Vec<(String, String, u32)>>();

    #[allow(clippy::min_ident_chars)]
    editions_vector.sort_by(|a, b| a.0.cmp(&b.0));
    editions_vector.reverse();

    editions_vector
}

fn generate_msrv_pages(crates: &[Crate]) -> Result<(), Box<dyn Error>> {
    log::info!("start generate_msrv_pages");

    let mut editions: HashMap<String, u32> = HashMap::new();
    let mut rust_versions: HashMap<String, u32> = HashMap::new();
    let mut rust_dash_versions: HashMap<String, u32> = HashMap::new();

    for krate in crates {
        let key1 = krate
            .cargo
            .package
            .edition
            .as_ref()
            .map_or_else(|| String::from("na"), core::clone::Clone::clone);
        *editions.entry(key1).or_insert(0) += 1;

        let key2 = krate
            .cargo
            .package
            .rust_version
            .as_ref()
            .map_or_else(|| String::from("na"), core::clone::Clone::clone);
        *rust_versions.entry(key2).or_insert(0) += 1;

        let key3 = krate
            .cargo
            .package
            .rust_dash_version
            .as_ref()
            .map_or_else(|| String::from("na"), core::clone::Clone::clone);
        *rust_dash_versions.entry(key3).or_insert(0) += 1;
    }

    log::info!("editions {:#?}", editions);
    log::info!("rust_version {:#?}", rust_versions);
    log::info!("rust_dash_version {:#?}", rust_dash_versions);

    let editions_vector = vectorize(&editions);
    let rust_versions_vector = vectorize(&rust_versions);
    let rust_dash_versions_vector = vectorize(&rust_dash_versions);

    let partials = load_templates().unwrap();

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/msrv.html")
        .unwrap();

    let filename = get_site_folder().join("msrv.html");
    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   "Rust MSRV Stats",
        "total_crates": crates.len(),
        "editions": editions_vector,
        "rust_versions": rust_versions_vector,
        "rust_dash_versions": rust_dash_versions_vector,
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{html}").unwrap();

    list_crates_with_edition(editions_vector, crates)?;
    list_crates_with_rust_version(rust_versions_vector, crates)?;
    list_crates_with_rust_dash_version(rust_dash_versions_vector, crates)?;

    log::info!("end generate_msrv_pages");
    Ok(())
}

fn list_crates_with_rust_dash_version(
    rust_dash_versions_vector: Vec<(String, String, u32)>,
    crates: &[Crate],
) -> Result<(), Box<dyn Error>> {
    for rust_dash_version in rust_dash_versions_vector {
        render_filtered_crates(
            &format!("rust-dash-version-{}", rust_dash_version.1),
            &format!(
                "Crates with rust-version field being '{}'",
                rust_dash_version.0
            ),
            |krate| {
                rust_dash_version.0
                    == krate
                        .cargo
                        .package
                        .rust_dash_version
                        .as_ref()
                        .map_or_else(|| String::from("na"), core::clone::Clone::clone)
            },
            crates,
        )?;
    }
    Ok(())
}

fn list_crates_with_rust_version(
    rust_versions_vector: Vec<(String, String, u32)>,
    crates: &[Crate],
) -> Result<(), Box<dyn Error>> {
    for rust_version in rust_versions_vector {
        render_filtered_crates(
            &format!("rust-version-{}", rust_version.1),
            &format!("Crates with rust_version field being '{}'", rust_version.0),
            |krate| {
                rust_version.0
                    == krate
                        .cargo
                        .package
                        .rust_version
                        .as_ref()
                        .map_or_else(|| String::from("na"), core::clone::Clone::clone)
            },
            crates,
        )?;
    }

    Ok(())
}

fn list_crates_with_edition(
    editions_vector: Vec<(String, String, u32)>,
    crates: &[Crate],
) -> Result<(), Box<dyn Error>> {
    for edition in editions_vector {
        render_filtered_crates(
            &format!("edition-{}", edition.1),
            &format!("Crates with edition field being '{}'", edition.0),
            |krate| {
                edition.0
                    == krate
                        .cargo
                        .package
                        .edition
                        .as_ref()
                        .map_or_else(|| String::from("na"), core::clone::Clone::clone)
            },
            crates,
        )?;
    }

    Ok(())
}

fn generate_rustfmt_pages(
    number_of_crates: usize,
    with_rustfmt: usize,
    crates: &[Crate],
) -> Result<(), Box<dyn Error>> {
    static RE_KEY: Lazy<Regex> = Lazy::new(|| Regex::new("^[a-z_]+$").unwrap());
    static RE_VALUE: Lazy<Regex> = Lazy::new(|| Regex::new("^[0-9A-Za-z_]+$").unwrap());

    let rustfmt = load_collected_rustfmt();
    let mut count_by_key: HashMap<String, u32> = HashMap::new();
    let mut count_by_pair: HashMap<(String, String), u32> = HashMap::new();

    #[allow(clippy::explicit_iter_loop)] // TODO
    #[allow(clippy::pattern_type_mismatch)] // TODO
    for (key, value, _krate) in rustfmt.iter() {
        *count_by_key.entry(key.to_owned()).or_insert(0) += 1;
        *count_by_pair
            .entry((key.to_owned(), value.to_owned()))
            .or_insert(0) += 1;
    }
    let mut count_by_key_vector = count_by_key
        .iter()
        //.map(|pair| pair)
        .collect::<Vec<(&String, &u32)>>();
    #[allow(clippy::min_ident_chars)]
    count_by_key_vector.sort_by_key(|f| f.1);
    count_by_key_vector.reverse();

    let mut count_by_pair_vector = count_by_pair
        .iter()
        .map(|pair| (&pair.0 .0, &pair.0 .1, pair.1))
        .collect::<Vec<(&String, &String, &u32)>>();
    #[allow(clippy::min_ident_chars)]
    count_by_pair_vector.sort_by(|a, b| a.0.partial_cmp(b.0).unwrap());
    //count_by_pair.reverse();

    #[allow(clippy::pattern_type_mismatch)] // TODO
    for (field, _count) in &count_by_key_vector {
        match RE_KEY.captures(field) {
            None => {
                log::error!("Invalid fmt key: {field}");
                continue;
            }
            Some(_) => {}
        };

        let crate_names = rustfmt
            .iter()
            .filter(|entry| &&entry.0 == field)
            .map(|entry| &entry.2)
            .collect::<Vec<&String>>();
        render_filtered_crates(
            &format!("rustfmt/{field}"),
            &format!("Crates using the {field} formatting option"),
            |krate| crate_names.contains(&&krate.name),
            crates,
        )?;
    }

    #[allow(clippy::pattern_type_mismatch)] // TODO
    for (field, value, _count) in &count_by_pair_vector {
        match RE_KEY.captures(field) {
            None => {
                log::error!("Invalid fmt key: {field}");
                continue;
            }
            Some(_) => match RE_VALUE.captures(value) {
                None => {
                    log::error!("Invalid fmt value: {field}   '{value}'");
                    continue;
                }
                Some(_) => {}
            },
        };

        let crate_names = rustfmt
            .iter()
            .filter(|entry| &&entry.0 == field && &&entry.1 == value)
            .map(|entry| &entry.2)
            .collect::<Vec<&String>>();

        render_filtered_crates(
            &format!("rustfmt/{field}_{value}"),
            &format!("Crates using the {field} formatting option set to {value}"),
            |krate| crate_names.contains(&&krate.name),
            crates,
        )?;
    }

    let partials = load_templates().unwrap();

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/rustfmt.html")
        .unwrap();

    let filename = get_site_folder().join("rustfmt/index.html");
    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   "Rustfmt Stats",
        "count_by_key": count_by_key_vector,
        "count_by_pair": count_by_pair_vector,
        "number_of_crates": number_of_crates,
        "with_rustfmt": with_rustfmt,
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{html}").unwrap();

    Ok(())
}

#[test]
fn check_load_templates() {
    let _partials = load_templates();
}

#[test]
fn test_get_repo_types() {
    let _repos = get_repo_types();
}
