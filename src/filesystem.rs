use std::{ffi::OsString, fs, io};

use regex::Regex;

pub fn create_dir_if_not_exists(dir: &str) -> io::Result<()> {
    fs::create_dir(dir)?;
    Ok(())
}

pub fn parse_file(file_sel: &str, file_name: &str) -> bool {
    match Regex::new(file_sel) {
        Ok(reg) => reg.is_match(&format!(r"{file_name}")),
        Err(err) => {
            eprintln!("Failed to parse regular expression! {err}");
            false
        }
    }
}

pub fn list_files(path: &str) -> io::Result<Vec<OsString>> {
    let entries = fs::read_dir(path)?
        .filter_map(|res| match res {
            Ok(e) => Some(e.file_name()),
            Err(err) => {
                eprintln!("Failed to read file! {err}");
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(entries)
}
