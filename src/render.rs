use chrono::prelude::{DateTime, Utc};
use liquid_filter_commafy::Commafy;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use crate::{Crate, CratesByOwner, Partials, Repo, User, PAGE_SIZE, VERSION};
use rust_digger::{get_owner_and_repo, percentage};

const URL: &str = "https://rust-digger.code-maven.com";

pub fn render_list_crates_by_repo(repos: &Vec<Repo>) -> Result<(), Box<dyn Error>> {
    log::info!("render_list_crates_by_repo start");
    for repo in repos {
        // dbg!(&repo);
        render_list_page(
            &format!("_site/vcs/{}.html", repo.name),
            &format!("Crates in {}", repo.display),
            &repo.name,
            &repo.crates,
        )?;
    }
    log::info!("render_list_crates_by_repo end");
    Ok(())
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

    let filename = "_site/vcs/index.html";
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

        let mut file = File::create(format!("_site/{}.html", page.0)).unwrap();
        writeln!(&mut file, "{html}").unwrap();
    }
    log::info!("render_static_pages end");
    Ok(())
}

pub fn render_list_page(
    filename: &String,
    title: &String,
    preface: &String,
    crates: &[Crate],
) -> Result<(), Box<dyn Error>> {
    // log::info!("render {filename}");

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
        "preface": preface,
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

    let mut file = File::create(filename).unwrap();
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
            Path::new("_site").join(entry.path().strip_prefix("templates/").unwrap().as_os_str());
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
        let filename = format!("_site/crates/{}.html", krate.name);
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
                user.count = selected_crates.len() as u16;
                //users_with_crates.push(user);

                #[allow(clippy::min_ident_chars)]
                selected_crates.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                let filename = format!("_site/users/{}.html", user.gh_login.to_ascii_lowercase());
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

    let filename = "_site/users/index.html";
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

fn render_stats_page(crates: usize, repos: &Vec<Repo>, stats: &HashMap<&str, usize>) {
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

    let filename = "_site/stats.html";
    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   "Rust Digger Stats",
        //"user":    user,
        //"crate":   krate,
        "total": crates,
        "repos": repos,
        "percentage": perc,
        "stats": stats,
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{html}").unwrap();
}

fn create_folders() {
    let _res = fs::create_dir_all("_site");
    let _res = fs::create_dir_all("_site/crates");
    let _res = fs::create_dir_all("_site/users");
    let _res = fs::create_dir_all("_site/news");
    let _res = fs::create_dir_all("_site/vcs");
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
    let paths = collect_paths(Path::new("_site"));
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
    let mut file = File::create("_site/sitemap.xml").unwrap();
    writeln!(&mut file, "{html}").unwrap();
}

pub fn generate_robots_txt() {
    let text = format!("Sitemap: {URL}/sitemap.xml\n\nUser-agent: *\n");
    let mut file = File::create("_site/robots.txt").unwrap();
    writeln!(&mut file, "{text}").unwrap();
}

/// Generate various lists of crates:
/// Filter the crates according to various rules and render them using `render_filtered_crates`.
/// Then using the numbers returned by that function generate the stats page.
pub fn generate_pages(
    crates: &[Crate],
    repos: &Vec<Repo>,
    no_repo: usize,
) -> Result<(), Box<dyn Error>> {
    log::info!("generate_pages");

    create_folders();

    fs::copy("digger.js", "_site/digger.js")?;

    render_list_crates_by_repo(repos)?;
    render_list_of_repos(repos);

    render_list_page(
        &String::from("_site/all.html"),
        &String::from("Rust Digger"),
        &String::from("all"),
        crates,
    )?;

    let github_but_no_ci = render_filtered_crates(
        &String::from("_site/github-but-no-ci.html"),
        &String::from("On GitHub but has no CI"),
        &String::from("github-but-no-ci"),
        crates,
        |krate| on_github_but_no_ci(krate),
    )?;

    let gitlab_but_no_ci = render_filtered_crates(
        &String::from("_site/gitlab-but-no-ci.html"),
        &String::from("On GitLab but has no CI"),
        &String::from("gitlab-but-no-ci"),
        crates,
        |krate| on_gitlab_but_no_ci(krate),
    )?;

    let home_page_but_no_repo = render_filtered_crates(
        &String::from("_site/has-homepage-but-no-repo.html"),
        &String::from("Has homepage, but no repository"),
        &String::from("has-homepage-but-no-repo"),
        crates,
        |krate| has_homepage_no_repo(krate),
    )?;

    let no_homepage_no_repo_crates = render_filtered_crates(
        &String::from("_site/no-homepage-no-repo.html"),
        &String::from("No repository, no homepage"),
        &String::from("no-homepage-no-repo"),
        crates,
        |krate| no_homepage_no_repo(krate),
    )?;

    let crates_without_owner_name = render_filtered_crates(
        &String::from("_site/crates-without-owner-name.html"),
        &String::from("Crates without owner name"),
        &String::from("crates-without-owner-name"),
        crates,
        |krate| krate.owner_name.is_empty(),
    )
    .unwrap();

    render_filtered_crates(
        &String::from("_site/crates-without-owner.html"),
        &String::from("Crates without owner"),
        &String::from("crates-without-owner"),
        crates,
        |krate| krate.owner_name.is_empty() && krate.owner_gh_login.is_empty(),
    )?;

    //log::info!("repos: {:?}", repos);

    let stats = HashMap::from([
        ("crates_without_owner_name", crates_without_owner_name),
        ("home_page_but_no_repo", home_page_but_no_repo),
        ("no_homepage_no_repo_crates", no_homepage_no_repo_crates),
        ("github_but_no_ci", github_but_no_ci),
        ("gitlab_but_no_ci", gitlab_but_no_ci),
        ("no_repo", no_repo),
    ]);

    render_stats_page(crates.len(), repos, &stats);

    Ok(())
}

fn render_filtered_crates(
    filename: &String,
    title: &String,
    preface: &String,
    crates: &[Crate],
    cond: fn(&&Crate) -> bool,
) -> Result<usize, Box<dyn Error>> {
    let filtered_crates = crates.iter().filter(cond).cloned().collect::<Vec<Crate>>();
    render_list_page(filename, title, preface, &filtered_crates)?;
    Ok(filtered_crates.len())
}

fn no_homepage_no_repo(w: &Crate) -> bool {
    w.homepage.is_empty() && w.repository.is_empty()
}

fn has_homepage_no_repo(w: &Crate) -> bool {
    !w.homepage.is_empty() && w.repository.is_empty()
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

#[test]
fn check_load_templates() {
    let _partials = load_templates();
}
