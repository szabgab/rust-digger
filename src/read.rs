use std::collections::HashMap;
use std::fs::File;

use crate::{CrateOwner, CratesByOwner, Owners, User, Users};

pub fn read_users(limit: i32) -> Users {
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

pub fn read_crate_owners(limit: i32) -> (Owners, CratesByOwner) {
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
