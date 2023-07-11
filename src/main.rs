use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
//use std::io::Read;
use std::collections::HashMap;

use chrono::prelude::*;
use handlebars::Handlebars;
use serde_json::json;
use serde_json::Value;

const VERSION: &str = env!("CARGO_PKG_VERSION");

struct Repo<'a> {
    name: &'a str,
    url: &'a str,
}

type Record = HashMap<String, String>;
type RepoPercentage<'a> = HashMap<&'a str, String>;
type Owners = HashMap<String, String>;
type CratesByOwner = HashMap<String, Vec<String>>;
type Users = HashMap<String, Record>;

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

    //crate_id,created_at,created_by,owner_id,owner_kind
    let mut owner_by_crate_id: Owners = HashMap::new();
    let mut crates_by_owner: CratesByOwner = HashMap::new();
    let result = read_csv_file("data/data/crate_owners.csv", limit);
    match result {
        Ok(rows) => {
            for row in rows {
                owner_by_crate_id.insert(row["crate_id"].clone(), row["owner_id"].clone());
                crates_by_owner.entry(row["owner_id"].clone()).or_insert(vec![]);
            }
        },
        Err(err) => panic!("Error: {}", err),
    };

    let users: Users = read_users(limit);
    let crates: Vec<Record> = read_crates(limit);

    match generate_pages(&crates, &users, &owner_by_crate_id, &crates_by_owner) {
        Ok(_) => {},
        Err(err) => panic!("Error: {}", err)
    }

    log::info!("Ending the Rust Digger");
}

fn render(reg: &Handlebars, template: &String, filename: &String, title: &String, params: &Value) -> Result<(), Box<dyn Error>> {
    // log::info!("render {filename}");

    let utc: DateTime<Utc> = Utc::now();
    let mut data = params.clone();
    data["version"] = json!(format!("{VERSION}"));
    data["utc"]     = json!(format!("{}", utc));
    data["title"]   = json!(title);
    data["parent"]  = json!("layout");

    let res = reg.render(template, &data);

    let mut file = File::create(filename).unwrap();
    match res {
        Ok(html) => writeln!(&mut file, "{}", html).unwrap(),
        Err(error) => println!("{}", error)
    }
    Ok(())
}

fn has_repo(w: &Record) -> bool {
    w["repository"] != ""
}
fn has_homepage_no_repo(w: &Record) -> bool {
    w["homepage"] != "" && w["repository"] == ""
}


fn get_repo_types(rows: &Vec<Record>) -> (HashMap<&str, usize>, RepoPercentage, Vec<&Record>) {
    let mut other: Vec<&Record> = vec![]; //&Vec<&HashMap<String, String>>;
    let repos = vec![
        Repo {name: "github",       url: "https://github.com/"},
        Repo {name: "gitlab",       url: "https://gitlab.com/"},
        Repo {name: "codeberg",     url: "https://codeberg.org/"},
        Repo {name: "gitee",        url: "https://gitee.com/"},
        Repo {name: "torproject",   url: "https://gitlab.torproject.org/"},
        Repo {name: "freedesktop",  url: "https://gitlab.freedesktop.org/"},
        Repo {name: "wikimedia",    url: "https://gitlab.wikimedia.org/"},
        Repo {name: "e3t",          url: "https://git.e3t.cc/"},
        Repo {name: "srht",         url: "https://git.sr.ht/"},
        Repo {name: "openprivacy",  url: "https://git.openprivacy.ca/"},
        Repo {name: "cronce",       url: "https://gitlab.cronce.io/"},
        Repo {name: "gnome",        url: "https://gitlab.gnome.org/"},
    ];
    let mut repo_type:HashMap<&str, usize> = HashMap::from([
        ("no_repo", 0),
        ("other", 0),
    ]);
    let mut repo_percentage:RepoPercentage = HashMap::new();
    for repo in &repos {
        repo_type.insert(repo.name, 0);
    }

    'outer: for row in rows {
        if row["repository"] == "" {
            *repo_type.entry("no_repo").or_insert(0) += 1;
            continue;
        }
        for repo in &repos {
            if row["repository"].starts_with(repo.url) {
                *repo_type.entry(&repo.name).or_insert(0) += 1;
                continue 'outer;
            }
        }
        *repo_type.entry("other").or_insert(0) += 1;
        other.push(row);
    }

    for repo in repos {
        repo_percentage.insert(repo.name, percentage(repo_type[repo.name], rows.len()));
    }
    repo_percentage.insert("other", percentage(repo_type["other"], rows.len()));
    repo_percentage.insert("no_repo", percentage(repo_type["no_repo"], rows.len()));

    (repo_type, repo_percentage, other)
}

fn percentage(num: usize, total: usize) -> String {
    let t = (10000 * num / total) as f32;
    (t / 100.0).to_string()
}


fn generate_user_pages(handlebar: &Handlebars, users: &Users) -> Result<(), Box<dyn Error>> {
    for (_uid, user) in users.iter() {
        render(&handlebar, &"user".to_string(), &format!("_site/users/{}.html", user["gh_login"].to_ascii_lowercase()), &user["name"], &json!({
            "user": user,
            }))?;
    }

    Ok(())
}

fn generate_crate_pages(
    handlebar: &Handlebars,
    rows: &Vec<Record>,
    users: &Users,
    owner_by_crate_id: &Owners,
    ) -> Result<(), Box<dyn Error>> {
    for row in rows {
        //dbg!(row);
        let crate_id = &row["id"];
        //dbg!(crate_id);
        let mut user: &Record = &HashMap::new();
        match owner_by_crate_id.get(crate_id) {
            Some(owner_id) => {
                //println!("owner_id: {owner_id}");
                match users.get(owner_id) {
                    Some(val) => {
                        user = val;
                        //println!("user: {:?}", user);
                    },
                    None => {
                        log::warn!("crate {crate_id} owner_id {owner_id} does not have a user");
                    },
                }
            },
            None => {
                log::warn!("crate {crate_id} does not have an owner");
            },
        };
        //let owner_id = &owner_by_crate_id[crate_id];
        //if owner_id != None {
        //    //dbg!(&owner_id);
        //    //dbg!(owner_id);
        //    //let user = &users[owner_id];
        //}
        //dbg!(user);
        render(&handlebar, &"crate".to_string(), &format!("_site/crates/{}.html", row["name"]), &row["name"], &json!({
            "crate": row,
            "user": user,
            }))?;
    }
    Ok(())
}


fn load_templates() -> Result<Handlebars<'static>, Box<dyn Error>> {
    log::info!("load_templates");

    let mut handlebar = Handlebars::new();
    handlebar.register_template_file("about", "templates/about.html")?;
    handlebar.register_template_file("list",  "templates/list.html")?;
    handlebar.register_template_file("stats", "templates/stats.html")?;
    handlebar.register_template_file("crate", "templates/crate.html")?;
    handlebar.register_template_file("user",  "templates/user.html")?;
    handlebar.register_template_file("layout", "templates/layout.html")?;

    Ok(handlebar)
}

fn generate_pages(
    rows :&Vec<Record>,
    users: &Users,
    owner_by_crate_id: &Owners,
    crates_by_owner: &CratesByOwner
    ) -> Result<(), Box<dyn Error>> {
    log::info!("generate_pages");

    let handlebar = match load_templates() {
        Ok(handlebar) => handlebar,
        Err(error) => panic!("Error loading templates {}", error),
    };

    // Create a folder _site
    let _res = fs::create_dir_all("_site");
    let _res = fs::create_dir_all("_site/crates");
    let _res = fs::create_dir_all("_site/users");

    let home_page_but_no_repo = rows.into_iter().filter(|w| has_homepage_no_repo(w)).collect::<Vec<&Record>>();
    let no_repo = rows.into_iter().filter(|w| !has_repo(w)).collect::<Vec<&Record>>();
    //dbg!(&no_repo[0..1]);

    let (repo_type, repo_percentage, other_repos): (HashMap<&str, usize>, RepoPercentage, Vec<&Record>) = get_repo_types(&rows);

    const PAGE_SIZE: usize = 100;

    let page_size = if rows.len() > PAGE_SIZE { PAGE_SIZE } else { rows.len() };
    render(&handlebar, &"list".to_string(), &"_site/index.html".to_string(), &"Rust Digger".to_string(), &json!({
        "total": rows.len(),
        "rows": &rows[0..page_size],
    }))?;

    let page_size = if no_repo.len() > PAGE_SIZE { PAGE_SIZE } else { no_repo.len() };
    render(&handlebar, &"list".to_string(), &"_site/no-repo.html".to_string(), &"Missing repository".to_string(), &json!({
        "total": no_repo.len(),
        "rows": &no_repo[0..page_size],
    }))?;


    let page_size = if home_page_but_no_repo.len() > PAGE_SIZE { PAGE_SIZE } else { home_page_but_no_repo.len() };
    render(&handlebar, &"list".to_string(), &"_site/has-homepage-but-no-repo.html".to_string(), &"Missing repository".to_string(), &json!({
        "total": home_page_but_no_repo.len(),
        "rows": &home_page_but_no_repo[0..page_size],
    }))?;


    let page_size = if other_repos.len() > PAGE_SIZE { PAGE_SIZE } else { other_repos.len() };
    render(&handlebar, &"list".to_string(), &"_site/other-repos.html".to_string(), &"Unknown repositories".to_string(), &json!({
        "total": other_repos.len(),
        "rows": &other_repos[0..page_size],
    }))?;


    render(&handlebar, &"about".to_string(), &"_site/about.html".to_string(), &"About Rust Digger".to_string(), &json!({}))?;

    log::info!("{:?}", repo_type);
    log::info!("{:?}", repo_percentage);
    render(&handlebar, &"stats".to_string(), &"_site/stats.html".to_string(), &"Rust Digger Stats".to_string(), &json!({
        "total": rows.len(),
        "no_repo": no_repo.len(),
        "no_repo_percentage": percentage(no_repo.len(), rows.len()),
        "repo_type": repo_type,
        "repo_percentage": repo_percentage,
        "home_page_but_no_repo": home_page_but_no_repo.len(),
        "home_page_but_no_repo_percentage":  percentage(home_page_but_no_repo.len(), rows.len()),
        }))?;

    generate_crate_pages(&handlebar, &rows, &users, &owner_by_crate_id)?;

    generate_user_pages(&handlebar, &users)?;

    Ok(())
}

fn read_crates(limit: i32) -> Vec<Record> {
    let crates: Vec<Record>;
    let result = read_csv_file("data/data/crates.csv", limit);
    match result {
        Ok(mut rows) => {
            rows.sort_by(|a, b| b["updated_at"].cmp(&a["updated_at"]));
            crates = rows;
        },
        Err(err) => panic!("Error: {}", err)
    }
    crates
}


fn read_users(limit: i32) -> Users {
    let mut users: Users = HashMap::new();
    let result = read_csv_file("data/data/users.csv", limit);
    match result {
        Ok(rows) => {
            for row in rows {
                //dbg!(&row);
                //dbg!(&row["id"]);
                users.insert(row["id"].clone(), row);
            }
        }
        Err(err) => panic!("Error: {}", err)
    }
    //dbg!(users);
    users
}


fn read_csv_file(filepath: &str, limit: i32) -> Result<Vec<Record>, Box<dyn Error>> {
    log::info!("Start reading {}", filepath);
    let mut records:Vec<Record> = vec![];
    let mut count = 0;
    match File::open(filepath.to_string()) {
        Ok(file) => {
            //let mut content = String::new();
            //file.read_to_string(&mut content).unwrap();
            let mut rdr = csv::Reader::from_reader(file);
            for result in rdr.deserialize() {
                count += 1;
                if limit > 0 && count >= limit {
                    log::info!("Limit of {limit} reached");
                    break
                }
                let record: Record = result?;
                records.push(record);
            }
        },
        Err(error) => panic!("Error opening file {}: {}", filepath, error),
    }

    log::info!("Finished reading {filepath}");
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_repo() {
        let x: Record = HashMap::from([("repository".to_string(), "https://github.com/szabgab/rust-digger".to_string())]);
        assert!(has_repo(&x));

        let x: Record = HashMap::from([("repository".to_string(), "".to_string())]);
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
