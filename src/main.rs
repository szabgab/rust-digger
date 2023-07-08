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

type Record = HashMap<String, String>;

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

    let mut users: HashMap<String, Record> = HashMap::new();
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

    let result = read_csv_file("data/data/crates.csv", limit);
    match result {
        Ok(mut rows) => {
            rows.sort_by(|a, b| b["updated_at"].cmp(&a["updated_at"]));
            match generate_pages(&rows, &users) {
                Ok(_) => {},
                Err(err) => panic!("Error: {}", err)
            }
        },
        Err(err) => panic!("Error: {}", err)
    }

    log::info!("Ending the Rust Digger");
}

fn render(reg: &Handlebars, template: &String, filename: &String, title: &String, params: &Value) -> Result<(), Box<dyn Error>> {
    log::info!("render {filename}");

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
    w["repository"] == ""
}

fn get_repo_types(rows: &Vec<Record>) -> (HashMap<&str, usize>, Vec<&Record>) {
    let mut other: Vec<&Record> = vec![]; //&Vec<&HashMap<String, String>>;
    let mut repo_type:HashMap<&str, usize> = HashMap::from([
        ("no_repo", 0),
        ("GitHub", 0),
        ("GitLab", 0),
        ("other", 0),
    ]);
    for row in rows {
        if row["repository"] == "" {
            *repo_type.entry("no_repo").or_insert(0) += 1;
            continue;
        }
        if row["repository"].starts_with("https://github.com/") {
            *repo_type.entry("GitHub").or_insert(0) += 1;
            continue;
        }
        if row["repository"].starts_with("https://gitlab.com/") {
            *repo_type.entry("GitLab").or_insert(0) += 1;
            continue;
        }
        *repo_type.entry("other").or_insert(0) += 1;
        other.push(row);
    }

    *repo_type.entry("GitHub_percentage").or_insert(0) = 100 * repo_type["GitHub"] / rows.len();
    *repo_type.entry("GitLab_percentage").or_insert(0) = 100 * repo_type["GitLab"] / rows.len();
    *repo_type.entry("other_percentage").or_insert(0) = 100 * repo_type["other"] / rows.len();
    *repo_type.entry("no_repo_percentage").or_insert(0) = 100 * repo_type["no_repo"] / rows.len();

     (repo_type, other)
}

fn generate_user_pages(reg: &Handlebars, users: &HashMap<String, Record>) -> Result<(), Box<dyn Error>> {
    for (_uid, user) in users.iter() {
        render(&reg, &"user".to_string(), &format!("_site/users/{}.html", user["gh_login"].to_ascii_lowercase()), &user["name"], &json!({
            "user": user,
            }))?;
    }

    Ok(())
}

fn load_templates() -> Result<Handlebars<'static>, Box<dyn Error>> {
    log::info!("load_templates");

    let mut reg = Handlebars::new();
    reg.register_template_file("about", "templates/about.html")?;
    reg.register_template_file("index", "templates/index.html")?;
    reg.register_template_file("stats", "templates/stats.html")?;
    reg.register_template_file("crate", "templates/crate.html")?;
    reg.register_template_file("user",  "templates/user.html")?;
    reg.register_template_file("layout", "templates/layout.html")?;

    Ok(reg)
}

fn generate_pages(rows :&Vec<Record>, users: &HashMap<String, Record>) -> Result<(), Box<dyn Error>> {
    log::info!("generate_pages");

    let reg = match load_templates() {
        Ok(reg) => reg,
        Err(error) => panic!("Error loading templates {}", error),
    };

    // Create a folder _site
    let _res = fs::create_dir_all("_site");
    let _res = fs::create_dir_all("_site/crates");
    let _res = fs::create_dir_all("_site/users");

    let no_repo = rows.into_iter().filter(|w| has_repo(w)).collect::<Vec<&Record>>();
    //dbg!(&no_repo[0..1]);

    let (repo_type, other_repos): (HashMap<&str, usize>, Vec<&Record>) = get_repo_types(&rows);

    const PAGE_SIZE: usize = 100;

    let page_size = if rows.len() > PAGE_SIZE { PAGE_SIZE } else { rows.len() };
    render(&reg, &"index".to_string(), &"_site/index.html".to_string(), &"Rust Digger".to_string(), &json!({
        "total": rows.len(),
        "rows": &rows[0..page_size],
    }))?;

    let page_size = if no_repo.len() > PAGE_SIZE { PAGE_SIZE } else { no_repo.len() };
    render(&reg, &"index".to_string(), &"_site/no-repo.html".to_string(), &"Missing repository".to_string(), &json!({
        "total": no_repo.len(),
        "rows": &no_repo[0..page_size],
    }))?;

    let page_size = if other_repos.len() > PAGE_SIZE { PAGE_SIZE } else { other_repos.len() };
    render(&reg, &"index".to_string(), &"_site/other-repos.html".to_string(), &"Unknown repositories".to_string(), &json!({
        "total": other_repos.len(),
        "rows": &other_repos[0..page_size],
    }))?;


    render(&reg, &"about".to_string(), &"_site/about.html".to_string(), &"About Rust Digger".to_string(), &json!({}))?;

    log::info!("{:?}", repo_type);
    render(&reg, &"stats".to_string(), &"_site/stats.html".to_string(), &"Rust Digger Stats".to_string(), &json!({
        "total": rows.len(),
        "no_repo": no_repo.len(),
        "no_repo_percentage": 100*no_repo.len()/rows.len(),
        "repo_type": repo_type,
        }))?;

    for row in rows {
        render(&reg, &"crate".to_string(), &format!("_site/crates/{}.html", row["name"]), &row["name"], &json!({
            "crate": row,
            }))?;
    }

    generate_user_pages(&reg, &users)?;

    Ok(())
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
