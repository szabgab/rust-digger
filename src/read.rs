use std::collections::HashMap;
use std::error::Error;
use std::fs::File;

use rust_digger::{get_db_dump_folder, CrateOwner, CratesByOwner, Owners, Team, User};

pub fn read_teams(users: &mut Vec<User>, limit: u32) -> Result<(), Box<dyn Error>> {
    let filepath = get_db_dump_folder().join("data/teams.csv");
    log::info!("Start reading {:?}", filepath.display());
    let mut count = 0;

    let file = File::open(&filepath)?;
    let mut rdr = csv::Reader::from_reader(file);
    for result in rdr.deserialize() {
        count += 1;
        if limit > 0 && count >= limit {
            log::info!("Limit of {limit} reached");
            break;
        }
        let record: Team = result?;
        let user = User {
            gh_avatar: record.avatar,
            gh_login: record.login,
            gh_id: record.github_id,
            name: record.name,
            id: record.id,
            count: 0,
            //org_id: record.org_id
            //team: true
        };
        users.push(user);
    }

    log::info!("Finished reading {:?}", filepath.display());
    Ok(())
}

pub fn read_users(limit: u32) -> Result<Vec<User>, Box<dyn Error>> {
    let mut users: Vec<User> = vec![];
    let filepath = get_db_dump_folder().join("data/users.csv");
    log::info!("Start reading {:?}", filepath.display());
    let mut count = 0;

    let file = File::open(&filepath)?;
    let mut rdr = csv::Reader::from_reader(file);
    for result in rdr.deserialize() {
        count += 1;
        if limit > 0 && count >= limit {
            log::info!("Limit of {limit} reached");
            break;
        }
        let record: User = result?;
        users.push(record);
    }

    log::info!("Finished reading {:?}", filepath.display());
    Ok(users)
}

pub fn read_crate_owners() -> Result<(Owners, CratesByOwner), Box<dyn Error>> {
    //crate_id,created_at,created_by,owner_id,owner_kind
    let mut owner_by_crate_id: Owners = HashMap::new();
    let mut crates_by_owner: CratesByOwner = HashMap::new();
    let filepath = get_db_dump_folder().join("data/crate_owners.csv");
    log::info!("Start reading {:?}", filepath.display());

    let file = File::open(&filepath)?;
    let mut rdr = csv::Reader::from_reader(file);
    for result in rdr.deserialize() {
        let record: CrateOwner = result?;

        owner_by_crate_id.insert(record.crate_id.clone(), record.owner_id.clone());
        crates_by_owner.entry(record.owner_id.clone()).or_default();
        crates_by_owner
            .get_mut(&record.owner_id)
            .ok_or_else(|| format!("Could not find owner {}", &record.owner_id))?
            .push(record.crate_id.clone());
        //dbg!(&crates_by_owner[&record.owner_id]);
    }

    log::info!("Finished reading {:?}", filepath.display());

    Ok((owner_by_crate_id, crates_by_owner))
}
