use chrono::{DateTime, FixedOffset, Local};
use colored::Colorize;
use serde::Serialize;
use serde_json::Value;
use std::env::temp_dir;
use std::fs::{read_to_string, OpenOptions};
use std::io::Write;
use std::process;

pub fn edit_as_json<T: ?Sized, F>(val: &T, mut on_save: F) -> Result<(), String>
where
    T: Serialize,
    F: FnMut(Value) -> Result<(), String>,
{
    let mut temp_file_name = temp_dir();
    temp_file_name.push("sitch.json");
    let mut temp_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&temp_file_name)
        .map_err(|_err| {
            "Could not make a temporary file. Please make sure that the \
             current user has edit access to the system's config file."
                .to_owned()
        })?;

    temp_file
        .set_len(0)
        .map_err(|_err| "Could not empty the temp file.".to_owned())?;
    let contents = format!("{}\n", serde_json::to_string_pretty(val).unwrap());
    temp_file.write_all(contents.as_bytes()).unwrap();
    let editor = std::env::var("EDITOR").map_err(|_err| {
        "Could not find your preferred editor. Please set your \
         EDITOR environment variable when editing text."
            .to_owned()
    })?;
    process::Command::new(editor)
        .arg(&temp_file_name)
        .output()
        .map_err(|err| format!("An error occurred while editing the JSON object: {}", err))?;

    let edited_json = read_to_string(&temp_file_name)
        .map_err(|_| "Could not read temp file after editing. Did it get deleted?".to_owned())?;
    let json = serde_json::from_str(&edited_json).map_err(|_| {
        "The edited object could not be parsed as JSON. Please try again.".to_owned()
    })?;
    on_save(json)?;

    Ok(())
}

pub fn parse_date(date: &str) -> Result<DateTime<Local>, String> {
    match DateTime::<FixedOffset>::parse_from_rfc2822(date) {
        Ok(date) => Ok(date.with_timezone(&Local)),
        Err(err) => Err(format!("Could not parse date: {}", err)),
    }
}

pub fn update_message(
    num_updates: usize,
    title: &str,
    link: &str,
    pub_date: &DateTime<Local>,
) -> String {
    let datetime_format = "%B %-e, %Y at %-l:%M %p";
    let num_updates_str = if num_updates == 1 {
        "has been 1 update".to_owned()
    } else {
        format!("have been {} updates", num_updates)
    };
    let update_str = format!(
        "\"{}\" released on {}, found here: {}",
        title,
        pub_date.format(datetime_format),
        link.bright_blue()
    );

    format!(
        "There {}, {} was {}",
        num_updates_str,
        if num_updates == 1 {
            "it"
        } else {
            "the earliest"
        },
        update_str,
    )
}
