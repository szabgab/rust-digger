use chrono::prelude::{DateTime, Utc};
use liquid_filter_commafy::Commafy;
use rust_digger::build_path;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::{collected_data_root, Crate, CratesByOwner, Partials, Repo, User, PAGE_SIZE, VERSION};
use rust_digger::{get_owner_and_repo, percentage};

const URL: &str = "https://rust-digger.code-maven.com";

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

pub fn read_file(filename: &str) -> String {
    let mut content = String::new();
    match File::open(filename) {
        Ok(mut file) => {
            file.read_to_string(&mut content).unwrap();
        }
        Err(error) => {
            log::error!("Error opening file {}: {}", filename, error);
        }
    }
    content
}

pub fn load_templates() -> Result<Partials, Box<dyn Error>> {
    // log::info!("load_templates");

    let mut partials = Partials::empty();
    let filename = "templates/incl/header.html";
    partials.add(filename, read_file(filename));
    let filename = "templates/incl/footer.html";
    partials.add(filename, read_file(filename));
    let filename = "templates/incl/navigation.html";
    partials.add(filename, read_file(filename));
    let filename = "templates/incl/list_crates.html";
    partials.add(filename, read_file(filename));

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

    let mut filepath = get_site_folder().join(filename);
    filepath.set_extension("html");

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

    let path = Path::new("templates/news");
    for entry in path.read_dir().expect("read_dir call failed").flatten() {
        let partials = load_templates().unwrap();
        if entry.path().extension().unwrap() != "html" {
            continue;
        }

        log::info!("news file: {:?}", entry.path());
        log::info!("{:?}", entry.path().strip_prefix("templates/"));
        let output_path =
            get_site_folder().join(entry.path().strip_prefix("templates/").unwrap().as_os_str());
        let template = liquid::ParserBuilder::with_stdlib()
            .filter(Commafy)
            .partials(partials)
            .build()
            .unwrap()
            .parse_file(entry.path())
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

    //            },
    //            Err(error) => {
    //                log:error!("Error opening file {:?}: {}", file.as_os_str(), error);
    //            },
    //        }
    //    }
}

pub fn generate_crate_pages(crates: &Vec<Crate>) -> Result<(), Box<dyn Error>> {
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
        let globals = liquid::object!({
            "version": format!("{VERSION}"),
            "utc":     format!("{}", utc),
            "title":   &krate.name,
            "crate":   krate,
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
                    log::info!("crate_id: {}", &crate_id);
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
                let utc: DateTime<Utc> = Utc::now();
                let globals = liquid::object!({
                    "version": format!("{VERSION}"),
                    "utc":     format!("{}", utc),
                    "title":   &user.name,
                    "user":    user,
                    "crates":  selected_crates,
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

    log::info!("generate_user_pages end");
    Ok(())
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

fn render_stats_page(crates: usize, stats: &HashMap<&str, usize>) {
    log::info!("render_stats_page");
    let partials = load_templates().unwrap();

    let template = liquid::ParserBuilder::with_stdlib()
        .filter(Commafy)
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/stats.html")
        .unwrap();

    let vector = stats
        .iter()
        .map(|(field, value)| (field, percentage(*value, crates)))
        .collect::<Vec<(&&str, String)>>();
    let perc: HashMap<&&str, String> = HashMap::from_iter(vector);

    let filename = get_site_folder().join("stats.html");
    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   "Rust Digger Stats",
        //"user":    user,
        //"crate":   krate,
        "total": crates,
        "percentage": perc,
        "stats": stats,
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{html}").unwrap();
}

pub fn create_folders() {
    let _res = fs::create_dir_all(get_site_folder());
    for folder in ["crates", "users", "news", "vcs", "rustfmt"] {
        let _res = fs::create_dir_all(get_site_folder().join(folder));
    }
}

fn collect_paths(root: &Path) -> Vec<String> {
    log::info!("collect_paths  from {:?}", root);

    let mut paths: Vec<String> = vec![];
    for entry in root.read_dir().expect("failed") {
        //log::info!("{}", &format!("{}", entry.unwrap().path().display())[5..])
        //paths.push(format!("{}", entry.unwrap().path().display())[5..].to_string().clone());
        let path = entry.as_ref().unwrap().path();
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
        crates,
        |krate| krate.repository.is_empty(),
    )?;

    let other_repo_count = render_filtered_crates(
        "vcs/other-repos",
        "Crates with other repositories we don't recognize",
        crates,
        |krate| {
            !(krate.repository.is_empty()
                || repos
                    .iter()
                    .any(|repo| krate.repository.starts_with(&repo.url)))
        },
    )?;

    repos = repos
        .into_iter()
        .map(|mut repo| {
            let count = render_filtered_crates(
                &format!("vcs/{}", &repo.name),
                &format!("Crates in {}", repo.display),
                crates,
                |krate| krate.repository.starts_with(&repo.url),
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

/// Generate various lists of crates:
/// Filter the crates according to various rules and render them using `render_filtered_crates`.
/// Then using the numbers returned by that function generate the stats page.
pub fn generate_pages(crates: &[Crate]) -> Result<(), Box<dyn Error>> {
    log::info!("generate_pages");

    fs::copy("digger.js", get_site_folder().join("digger.js"))?;

    let no_repo = collect_repos(crates)?;

    let _all = render_filtered_crates("all", "Rust Digger", crates, |_krate| true)?;

    let has_cargo_toml_in_root = render_filtered_crates(
        "has-cargo-toml-in-root",
        "Has Cargo.toml file in the root",
        crates,
        |krate| krate.details.cargo_toml_in_root,
    )?;

    let has_no_cargo_toml_in_root = render_filtered_crates(
        "has-no-cargo-toml-in-root",
        "Has no Cargo.toml file in the root",
        crates,
        |krate| !krate.details.cargo_toml_in_root,
    )?;

    let has_rustfmt_toml = render_filtered_crates(
        "has-rustfmt-toml",
        "Has rustfmt.toml file",
        crates,
        |krate| krate.details.has_rustfmt_toml,
    )?;

    let has_dot_rustfmt_toml = render_filtered_crates(
        "has-dot-rustfmt-toml",
        "Has .rustfmt.toml file",
        crates,
        |krate| krate.details.has_dot_rustfmt_toml,
    )?;

    let has_both_rustfmt_toml = render_filtered_crates(
        "has-both-rustfmt-toml",
        "Has both rustfmt.toml and .rustfmt.toml file",
        crates,
        |krate| krate.details.has_rustfmt_toml && krate.details.has_dot_rustfmt_toml,
    )?;

    let github_but_no_ci = render_filtered_crates(
        "github-but-no-ci",
        "On GitHub but has no CI",
        crates,
        |krate| on_github_but_no_ci(krate),
    )?;

    let gitlab_but_no_ci = render_filtered_crates(
        "gitlab-but-no-ci",
        "On GitLab but has no CI",
        crates,
        |krate| on_gitlab_but_no_ci(krate),
    )?;

    let home_page_but_no_repo = render_filtered_crates(
        "has-homepage-but-no-repo",
        "Has homepage, but no repository",
        crates,
        |krate| has_homepage_no_repo(krate),
    )?;

    let no_homepage_no_repo_crates = render_filtered_crates(
        "no-homepage-no-repo",
        "No repository, no homepage",
        crates,
        |krate| no_homepage_no_repo(krate),
    )?;

    let crates_without_owner_name = render_filtered_crates(
        "crates-without-owner-name",
        "Crates without owner name",
        crates,
        |krate| no_owner_name(krate),
    )
    .unwrap();

    let crates_without_owner = render_filtered_crates(
        "crates-without-owner",
        "Crates without owner",
        crates,
        |krate| crate_has_no_owner(krate),
    )?;

    let stats = HashMap::from([
        ("crates_without_owner", crates_without_owner),
        ("crates_without_owner_name", crates_without_owner_name),
        ("home_page_but_no_repo", home_page_but_no_repo),
        ("no_homepage_no_repo_crates", no_homepage_no_repo_crates),
        ("github_but_no_ci", github_but_no_ci),
        ("gitlab_but_no_ci", gitlab_but_no_ci),
        ("no_repo", no_repo),
        ("has_rustfmt_toml", has_rustfmt_toml),
        ("has_dot_rustfmt_toml", has_dot_rustfmt_toml),
        ("has_both_rustfmt_toml", has_both_rustfmt_toml),
        ("has_cargo_toml_in_root", has_cargo_toml_in_root),
        ("has_no_cargo_toml_in_root", has_no_cargo_toml_in_root),
    ]);

    render_stats_page(crates.len(), &stats);
    generate_rustfmt_pages(crates.len(), &stats, crates)?;

    Ok(())
}

fn render_filtered_crates(
    filename: &str,
    title: &str,
    crates: &[Crate],
    cond: impl Fn(&&Crate) -> bool,
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

fn no_homepage_no_repo(krate: &Crate) -> bool {
    krate.homepage.is_empty() && krate.repository.is_empty()
}

fn has_homepage_no_repo(krate: &Crate) -> bool {
    !krate.homepage.is_empty() && krate.repository.is_empty()
}

// fn has_repo(w: &Crate) -> bool {
//     w.repository != ""
// }
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

    if krate.details.has_github_action {
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

    if krate.details.has_gitlab_pipeline {
        return false;
    }

    true
}

fn no_owner_name(krate: &Crate) -> bool {
    krate.owner_name.is_empty()
}

fn crate_has_no_owner(krate: &Crate) -> bool {
    krate.owner_name.is_empty() && krate.owner_gh_login.is_empty()
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

fn generate_rustfmt_pages(
    number_of_crates: usize,
    stats: &HashMap<&str, usize>,
    crates: &[Crate],
) -> Result<(), Box<dyn Error>> {
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
    let mut count_by_key = count_by_key
        .iter()
        //.map(|pair| pair)
        .collect::<Vec<(&String, &u32)>>();
    #[allow(clippy::min_ident_chars)]
    count_by_key.sort_by_key(|f| f.1);
    count_by_key.reverse();

    let mut count_by_pair = count_by_pair
        .iter()
        .map(|pair| (&pair.0 .0, &pair.0 .1, pair.1))
        .collect::<Vec<(&String, &String, &u32)>>();
    #[allow(clippy::min_ident_chars)]
    count_by_pair.sort_by(|a, b| a.0.partial_cmp(b.0).unwrap());
    //count_by_pair.reverse();

    #[allow(clippy::pattern_type_mismatch)] // TODO
    for (field, _count) in &count_by_key {
        let crate_names = rustfmt
            .iter()
            .filter(|entry| &&entry.0 == field)
            .map(|entry| &entry.2)
            .collect::<Vec<&String>>();
        render_filtered_crates(
            &format!("rustfmt/{field}"),
            &format!("Crates using the {field} formatting option"),
            crates,
            |krate| crate_names.contains(&&krate.name),
        )?;
    }

    #[allow(clippy::pattern_type_mismatch)] // TODO
    for (field, value, _count) in &count_by_pair {
        let crate_names = rustfmt
            .iter()
            .filter(|entry| &&entry.0 == field && &&entry.1 == value)
            .map(|entry| &entry.2)
            .collect::<Vec<&String>>();
        render_filtered_crates(
            &format!("rustfmt/{field}_{value}"),
            &format!("Crates using the {field} formatting option set to {value}"),
            crates,
            |krate| crate_names.contains(&&krate.name),
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
        "count_by_key": count_by_key,
        "count_by_pair": count_by_pair,
        "stats": stats,
        "number_of_crates": number_of_crates,
        "with_rustfmt": stats["has_rustfmt_toml"] + stats["has_dot_rustfmt_toml"],
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
