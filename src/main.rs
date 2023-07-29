use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use chrono::prelude::*;

pub type Partials = liquid::partials::EagerCompiler<liquid::partials::InMemorySource>;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PAGE_SIZE: usize = 100;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Repo {
    display: String,
    name: String,
    url: String,
    count: usize,
    percentage: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Crate {
    created_at: String,
    description: String,
    documentation: String,
    downloads: String,
    homepage: String,
    id: String,
    max_upload_size: String,
    name: String,
    readme: String,
    repository: String,
    updated_at: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct User {
    gh_avatar: String,
    gh_id: String,
    gh_login: String,
    id: String,
    name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CrateOwner {
    crate_id: String,
    created_at: String,
    created_by: String,
    owner_id: String,
    owner_kind: String,
}

//type RepoPercentage<'a> = HashMap<&'a str, String>;
type Owners = HashMap<String, String>;
type CratesByOwner = HashMap<String, Vec<String>>;
type Users = HashMap<String, User>;

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    log::info!("Starting the Rust Digger");
    log::info!("{VERSION}");

    let args: Vec<String> = env::args().collect();
    let limit;
    if args.len() == 2 {
        limit = args[1].parse().expect("Could not convert to i32");
    } else {
        limit = 0;
    }
    log::info!("Limit {limit}");

    let (owner_by_crate_id, crates_by_owner): (Owners, CratesByOwner) = read_crate_owners(limit);
    let users: Users = read_users(limit);
    let crates: Vec<Crate> = read_crates(limit);
    //dbg!(&crates_by_owner);

    match generate_pages(&crates, &users, &owner_by_crate_id, &crates_by_owner) {
        Ok(_) => {}
        Err(err) => panic!("Error: {}", err),
    }

    log::info!("Ending the Rust Digger");
}

fn render_static_pages() -> Result<(), Box<dyn Error>> {
    log::info!("render_static_pages");

    let pages = vec![
        ("about", "About Rust Digger"),
        ("support", "Support Rust Digger"),
    ];

    for page in pages {
        let partials = match load_templates() {
            Ok(partials) => partials,
            Err(error) => panic!("Error loading templates {}", error),
        };

        let utc: DateTime<Utc> = Utc::now();
        let globals = liquid::object!({
            "version": format!("{VERSION}"),
            "utc":     format!("{}", utc),
            "title":   page.1,
        });

        let template = liquid::ParserBuilder::with_stdlib()
            .partials(partials)
            .build()
            .unwrap()
            .parse_file(format!("templates/{}.html", page.0))
            .unwrap();
        let html = template.render(&globals).unwrap();

        let mut file = File::create(format!("_site/{}.html", page.0)).unwrap();
        writeln!(&mut file, "{}", html).unwrap();
    }
    Ok(())
}

fn render_list_page(
    filename: &String,
    title: &String,
    crates: &Vec<&Crate>,
) -> Result<(), Box<dyn Error>> {
    // log::info!("render {filename}");

    let partials = match load_templates() {
        Ok(partials) => partials,
        Err(error) => panic!("Error loading templates {}", error),
    };

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
        "total":   crates.len(),
        "crates":  (&crates[0..page_size]).to_vec(),
    });

    let template = liquid::ParserBuilder::with_stdlib()
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/crate_list_page.html")
        .unwrap();
    let html = template.render(&globals).unwrap();

    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{}", html).unwrap();
    //match res {
    //    Ok(html) => writeln!(&mut file, "{}", html).unwrap(),
    //    Err(error) => println!("{}", error)
    //}
    Ok(())
}

// fn has_repo(w: &Crate) -> bool {
//     w.repository != ""
// }
fn has_homepage_no_repo(w: &Crate) -> bool {
    w.homepage != "" && w.repository == ""
}
fn no_homepage_no_repo(w: &Crate) -> bool {
    w.homepage == "" && w.repository == ""
}

fn get_repo_types() -> Vec<Repo> {
    let repos: Vec<Repo> = vec![
        Repo {
            display: "GitHub".to_string(),
            name: "github".to_string(),
            url: "https://github.com/".to_string(),
            count: 0,
            percentage: "0".to_string(),
        },
        Repo {
            display: "GitLab".to_string(),
            name: "gitlab".to_string(),
            url: "https://gitlab.com/".to_string(),
            count: 0,
            percentage: "0".to_string(),
        },
        Repo {
            display: "Codeberg".to_string(),
            name: "codeberg".to_string(),
            url: "https://codeberg.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
        },
        Repo {
            display: "Gitee".to_string(),
            name: "gitee".to_string(),
            url: "https://gitee.com/".to_string(),
            count: 0,
            percentage: "0".to_string(),
        },
        Repo {
            display: "Tor Project (GitLab)".to_string(),
            name: "torproject".to_string(),
            url: "https://gitlab.torproject.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
        },
        Repo {
            display: "Free Desktop (GitLab)".to_string(),
            name: "freedesktop".to_string(),
            url: "https://gitlab.freedesktop.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
        },
        Repo {
            display: "Wikimedia (GitLab)".to_string(),
            name: "wikimedia".to_string(),
            url: "https://gitlab.wikimedia.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
        },
        Repo {
            display: "e3t".to_string(),
            name: "e3t".to_string(),
            url: "https://git.e3t.cc/".to_string(),
            count: 0,
            percentage: "0".to_string(),
        },
        Repo {
            display: "srht".to_string(),
            name: "srht".to_string(),
            url: "https://git.sr.ht/".to_string(),
            count: 0,
            percentage: "0".to_string(),
        },
        Repo {
            display: "Open Privacy".to_string(),
            name: "openprivacy".to_string(),
            url: "https://git.openprivacy.ca/".to_string(),
            count: 0,
            percentage: "0".to_string(),
        },
        Repo {
            display: "Cronce (GitLab)".to_string(),
            name: "cronce".to_string(),
            url: "https://gitlab.cronce.io/".to_string(),
            count: 0,
            percentage: "0".to_string(),
        },
        Repo {
            display: "Gnome (GitLab)".to_string(),
            name: "gnome".to_string(),
            url: "https://gitlab.gnome.org/".to_string(),
            count: 0,
            percentage: "0".to_string(),
        },
    ];
    repos
}

fn collect_repos(crates: &Vec<Crate>) -> (Vec<&Crate>, Vec<Repo>, Vec<&Crate>) {
    let mut repos: Vec<Repo> = get_repo_types();
    let mut no_repo: Vec<&Crate> = vec![];
    let mut other_repo: Vec<&Crate> = vec![];

    for krate in crates {
        if krate.repository == "" {
            no_repo.push(krate);
            continue;
        }
        let mut matched = false;
        repos = repos.into_iter().map(|mut repo| {
            if krate.repository.starts_with(&repo.url) {
                repo.count += 1;
                matched = true;
            }
            repo
        }).collect();

        if !matched {
            other_repo.push(krate);
        }
    }

    repos.push(
        Repo {
            display: "No repo".to_string(),
            name: "no_repo".to_string(),
            url: "".to_string(),
            count: no_repo.len(),
            percentage: "0".to_string(),
        });

    repos.push(
        Repo {
            display: "Other repo".to_string(),
            name: "other_repo".to_string(),
            url: "".to_string(),
            count: other_repo.len(),
            percentage: "0".to_string(),
        });
    
    repos = repos.into_iter().map(|mut repo| {
        repo.percentage = percentage(repo.count, crates.len());
        repo
    }).collect();

    (no_repo, repos, other_repo)
}

fn percentage(num: usize, total: usize) -> String {
    let t = (10000 * num / total) as f32;
    (t / 100.0).to_string()
}

fn generate_user_pages(
    crates: &Vec<Crate>,
    users: &Users,
    crates_by_owner: &CratesByOwner,
) -> Result<(), Box<dyn Error>> {
    let partials = match load_templates() {
        Ok(partials) => partials,
        Err(error) => panic!("Error loading templates {}", error),
    };

    let template = liquid::ParserBuilder::with_stdlib()
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

    for (uid, user) in users.iter() {
        //dbg!(uid);
        let mut selected_crates: Vec<&Crate> = vec![];
        match crates_by_owner.get(uid) {
            Some(crate_ids) => {
                //dbg!(crate_ids);
                for crate_id in crate_ids {
                    //dbg!(&crate_id);
                    //dbg!(&crate_by_id[crate_id.as_str()]);
                    //dbg!(&crate_by_id.get(&crate_id.clone()));
                    selected_crates.push(&crate_by_id[crate_id.as_str()]);
                }
            }
            None => {
                log::warn!("user {uid} does not have crates");
            }
        }

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
        writeln!(&mut file, "{}", html).unwrap();
    }

    Ok(())
}

fn generate_crate_pages(
    crates: &Vec<Crate>,
    users: &Users,
    owner_by_crate_id: &Owners,
) -> Result<(), Box<dyn Error>> {
    let partials = match load_templates() {
        Ok(partials) => partials,
        Err(error) => panic!("Error loading templates {}", error),
    };

    let template = liquid::ParserBuilder::with_stdlib()
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/crate.html")
        .unwrap();

    for krate in crates {
        //dbg!(crate);
        let crate_id = &krate.id;
        //dbg!(crate_id);
        let mut user: &User = &User {
            gh_avatar: "".to_string(),
            gh_id: "".to_string(),
            gh_login: "".to_string(),
            id: "".to_string(),
            name: "".to_string(),
        };
        match owner_by_crate_id.get(crate_id) {
            Some(owner_id) => {
                //println!("owner_id: {owner_id}");
                match users.get(owner_id) {
                    Some(val) => {
                        user = val;
                        //println!("user: {:?}", user);
                    }
                    None => {
                        log::warn!("crate {crate_id} owner_id {owner_id} does not have a user");
                    }
                }
            }
            None => {
                log::warn!("crate {crate_id} does not have an owner");
            }
        };
        //let owner_id = &owner_by_crate_id[crate_id];
        //if owner_id != None {
        //    //dbg!(&owner_id);
        //    //dbg!(owner_id);
        //    //let user = &users[owner_id];
        //}
        //dbg!(user);
        let filename = format!("_site/crates/{}.html", krate.name);
        let utc: DateTime<Utc> = Utc::now();
        let globals = liquid::object!({
            "version": format!("{VERSION}"),
            "utc":     format!("{}", utc),
            "title":   &krate.name,
            "user":    user,
            "crate":   krate,
        });
        let html = template.render(&globals).unwrap();
        let mut file = File::create(filename).unwrap();
        writeln!(&mut file, "{}", html).unwrap();
    }
    Ok(())
}

fn load_templates() -> Result<Partials, Box<dyn Error>> {
    // log::info!("load_templates");

    let mut partials = Partials::empty();
    let filename = "templates/incl/header.html";
    partials.add(filename, read_file(filename));
    let filename = "templates/incl/footer.html";
    partials.add(filename, read_file(filename));
    let filename = "templates/incl/list_crates.html";
    partials.add(filename, read_file(filename));

    Ok(partials)
}

fn generate_pages(
    crates: &Vec<Crate>,
    users: &Users,
    owner_by_crate_id: &Owners,
    crates_by_owner: &CratesByOwner,
) -> Result<(), Box<dyn Error>> {
    log::info!("generate_pages");

    // Create a folder _site
    let _res = fs::create_dir_all("_site");
    let _res = fs::create_dir_all("_site/crates");
    let _res = fs::create_dir_all("_site/users");
    let _res = fs::create_dir_all("_site/news");

    fs::copy("digger.js", "_site/digger.js")?;

    let all_crates = crates.into_iter().collect::<Vec<&Crate>>();
    let home_page_but_no_repo = crates
        .into_iter()
        .filter(|w| has_homepage_no_repo(w))
        .collect::<Vec<&Crate>>();
    let no_homepage_no_repo_crates = crates
        .into_iter()
        .filter(|w| no_homepage_no_repo(w))
        .collect::<Vec<&Crate>>();
    // let no_repo = crates
    //     .into_iter()
    //     .filter(|w| !has_repo(w))
    //     .collect::<Vec<&Crate>>();
    let repo_with_http = crates
        .into_iter()
        .filter(|w| w.repository != "" && w.repository.starts_with("http://"))
        .collect::<Vec<&Crate>>();
    let github_with_www = crates
        .into_iter()
        .filter(|w| w.repository != "" && w.repository.contains("www.github.com"))
        .collect::<Vec<&Crate>>();
    //dbg!(&no_repo[0..1]);

    let (no_repo, repos, other_repos) = collect_repos(&crates);

    let mut partials = Partials::empty();
    let filename = "templates/incl/header.html";
    partials.add(filename, read_file(filename));
    let filename = "templates/incl/footer.html";
    partials.add(filename, read_file(filename));

    render_list_page(
        &"_site/index.html".to_string(),
        &"Rust Digger".to_string(),
        &all_crates,
    )?;

    render_list_page(
        &"_site/no-repo.html".to_string(),
        &"Has no repository".to_string(),
        &no_repo,
    )?;

    render_list_page(
        &"_site/repo-with-http.html".to_string(),
        &"Repository is unsecure".to_string(),
        &repo_with_http,
    )?;

    render_list_page(
        &"_site/github-with-www.html".to_string(),
        &"Github with www".to_string(),
        &github_with_www,
    )?;

    render_list_page(
        &"_site/has-homepage-but-no-repo.html".to_string(),
        &"Has homepage, but no repository".to_string(),
        &home_page_but_no_repo,
    )?;

    render_list_page(
        &"_site/no-homepage-no-repo.html".to_string(),
        &"No repository, no homepage".to_string(),
        &no_homepage_no_repo_crates,
    )?;

    render_list_page(
        &"_site/other-repos.html".to_string(),
        &"Other repositories we don't recognize".to_string(),
        &other_repos,
    )?;

    render_news_pages();

    render_about_page()?;

    //log::info!("repos: {:?}", repos);

    log::info!("render_stats_page");
    let partials = match load_templates() {
        Ok(partials) => partials,
        Err(error) => panic!("Error loading templates {}", error),
    };

    let template = liquid::ParserBuilder::with_stdlib()
        .partials(partials)
        .build()
        .unwrap()
        .parse_file("templates/stats.html")
        .unwrap();

    let filename = "_site/stats.html";
    let utc: DateTime<Utc> = Utc::now();
    let globals = liquid::object!({
        "version": format!("{VERSION}"),
        "utc":     format!("{}", utc),
        "title":   "Rust Digger Stats",
        //"user":    user,
        //"crate":   krate,
        "total": crates.len(),
        "repos": repos,
        "home_page_but_no_repo": home_page_but_no_repo.len(),
        "home_page_but_no_repo_percentage":  percentage(home_page_but_no_repo.len(), crates.len()),
        "no_homepage_no_repo_crates": no_homepage_no_repo_crates.len(),
        "no_homepage_no_repo_crates_percentage": percentage(no_homepage_no_repo_crates.len(), crates.len()),
        "repo_with_http": repo_with_http.len(),
        "repo_with_http_percentage": percentage(repo_with_http.len(), crates.len()),
        "github_with_www": github_with_www.len(),
        "github_with_www_percentage": percentage(github_with_www.len(), crates.len()),
    });
    let html = template.render(&globals).unwrap();
    let mut file = File::create(filename).unwrap();
    writeln!(&mut file, "{}", html).unwrap();

    generate_crate_pages(&crates, &users, &owner_by_crate_id)?;

    generate_user_pages(&crates, &users, &crates_by_owner)?;

    Ok(())
}

fn render_news_pages() {
    log::info!("render_news_pages");
    let utc: DateTime<Utc> = Utc::now();

    let path = Path::new("templates/news");
    for entry in path.read_dir().expect("read_dir call failed") {
        if let Ok(entry) = entry {
            let partials = match load_templates() {
                Ok(partials) => partials,
                Err(error) => panic!("Error loading templates {}", error),
            };
            if entry.path().extension().unwrap() != "html" {
                continue;
            }

            log::info!("news file: {:?}", entry.path());
            log::info!("{:?}", entry.path().strip_prefix("templates/"));
            let output_path = Path::new("_site")
                .join(entry.path().strip_prefix("templates/").unwrap().as_os_str());
            let template = liquid::ParserBuilder::with_stdlib()
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
            writeln!(&mut file, "{}", html).unwrap();
        }
    }

    //            },
    //            Err(error) => {
    //                println!("Error opening file {:?}: {}", file.as_os_str(), error);
    //            },
    //        }
    //    }
}

fn read_crate_owners(limit: i32) -> (Owners, CratesByOwner) {
    //crate_id,created_at,created_by,owner_id,owner_kind
    let mut owner_by_crate_id: Owners = HashMap::new();
    let mut crates_by_owner: CratesByOwner = HashMap::new();
    let filepath = "data/data/crate_owners.csv";
    log::info!("Start reading {}", filepath);
    let mut count = 0;
    match File::open(filepath.to_string()) {
        Ok(file) => {
            let mut rdr = csv::Reader::from_reader(file);
            for result in rdr.deserialize() {
                count += 1;
                if limit > 0 && count >= limit {
                    log::info!("Limit of {limit} reached");
                    break;
                }
                let record: CrateOwner = match result {
                    Ok(value) => value,
                    Err(error) => panic!("Error {}", error),
                };
                owner_by_crate_id.insert(record.crate_id.clone(), record.owner_id.clone());
                crates_by_owner
                    .entry(record.owner_id.clone())
                    .or_insert(vec![]);
                let _ = &crates_by_owner
                    .get_mut(&record.owner_id)
                    .unwrap()
                    .push(record.crate_id.clone());
                //dbg!(&crates_by_owner[&record.owner_id]);
            }
        }
        Err(error) => panic!("Error opening file {}: {}", filepath, error),
    }

    log::info!("Finished reading {filepath}");

    (owner_by_crate_id, crates_by_owner)
}

fn read_crates(limit: i32) -> Vec<Crate> {
    let filepath = "data/data/crates.csv";
    log::info!("Start reading {}", filepath);
    let mut crates: Vec<Crate> = vec![];
    let mut count = 0;
    match File::open(filepath.to_string()) {
        Ok(file) => {
            let mut rdr = csv::Reader::from_reader(file);
            for result in rdr.deserialize() {
                count += 1;
                if limit > 0 && count >= limit {
                    log::info!("Limit of {limit} reached");
                    break;
                }
                let record: Crate = match result {
                    Ok(value) => value,
                    Err(error) => panic!("error: {}", error),
                };
                crates.push(record);
            }
            crates.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        }
        Err(error) => panic!("Error opening file {}: {}", filepath, error),
    }

    log::info!("Finished reading {filepath}");
    crates
}

fn read_users(limit: i32) -> Users {
    let mut users: Users = HashMap::new();
    let filepath = "data/data/users.csv";
    log::info!("Start reading {}", filepath);
    let mut count = 0;
    match File::open(filepath.to_string()) {
        Ok(file) => {
            let mut rdr = csv::Reader::from_reader(file);
            for result in rdr.deserialize() {
                count += 1;
                if limit > 0 && count >= limit {
                    log::info!("Limit of {limit} reached");
                    break;
                }
                let record: User = match result {
                    Ok(value) => value,
                    Err(err) => panic!("Error: {}", err),
                };
                users.insert(record.id.clone(), record);
            }
        }
        Err(error) => panic!("Error opening file {}: {}", filepath, error),
    }

    log::info!("Finished reading {filepath}");
    users
}

fn read_file(filename: &str) -> String {
    let mut content = String::new();
    match File::open(filename) {
        Ok(mut file) => {
            file.read_to_string(&mut content).unwrap();
        }
        Err(error) => {
            println!("Error opening file {}: {}", filename, error);
        }
    }
    content
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_repo() {
        let x = Crate {
            created_at: "".to_string(),
            description: "".to_string(),
            documentation: "".to_string(),
            downloads: "".to_string(),
            homepage: "".to_string(),
            id: "".to_string(),
            max_upload_size: "".to_string(),
            name: "".to_string(),
            readme: "".to_string(),
            repository: "https://github.com/szabgab/rust-digger".to_string(),
            updated_at: "".to_string(),
        };
        assert!(has_repo(&x));

        let x = Crate {
            created_at: "".to_string(),
            description: "".to_string(),
            documentation: "".to_string(),
            downloads: "".to_string(),
            homepage: "".to_string(),
            id: "".to_string(),
            max_upload_size: "".to_string(),
            name: "".to_string(),
            readme: "".to_string(),
            repository: "".to_string(),
            updated_at: "".to_string(),
        };
        assert!(!has_repo(&x));
    }

    #[test]
    fn test_percentage() {
        assert_eq!(percentage(20, 100), "20");
        assert_eq!(percentage(5, 20), "25");
        assert_eq!(percentage(1234, 10000), "12.34");
        assert_eq!(percentage(1234567, 10000000), "12.34");
    }
}
