use std::fs::{File, OpenOptions};

use serde_yaml::{self};

use crate::notifications::Notification;

pub fn read(fpath: &String) -> Result<Vec<Notification>, String> {
    let open_file =
        File::open(fpath.clone()).map_err(|e| format!("unable to read file '{}': {}", fpath, e))?;
    let notifications: Vec<Notification> = serde_yaml::from_reader(open_file)
        .map_err(|e| format!("unable to parse config file '{}': {}", fpath, e))?;
    Ok(notifications)
}

pub fn write(n: &Vec<Notification>, fpath: &String) -> Result<(), String> {
    let f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(fpath)
        .expect("Couldn't open file");
    serde_yaml::to_writer(f, n)
        .map_err(|e| format!("unable to parse config file '{}': {}", fpath, e))?;
    Ok(())
}
