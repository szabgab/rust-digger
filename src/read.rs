use std::collections::HashMap;
use std::fs::File;

use crate::{User, Users};

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
