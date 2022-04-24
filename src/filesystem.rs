use std::{ffi::OsString, fs, io};

use regex::Regex;

pub fn create_dir_if_not_exists(dir: &str) -> io::Result<()> {
    fs::create_dir(dir)?;
    Ok(())
}

fn parse_file(file_sel: &str, file_name: &str) -> bool {
    match Regex::new(file_sel) {
        Ok(reg) => reg.is_match(&format!(r"{file_name}")),
        Err(err) => {
            eprintln!("Failed to parse regular expression! {err}");
            false
        }
    }
}

pub fn filter_files(files: Vec<OsString>, file_sel: &str) -> Vec<OsString> {
    files
        .iter()
        .cloned()
        .filter(|f| match f.to_str() {
            Some(file_name) => parse_file(file_sel, file_name),
            None => false,
        })
        .collect::<Vec<_>>()
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
