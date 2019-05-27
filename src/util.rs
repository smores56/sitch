//! Some miscellaneous utility functions used throughout sitch.

use serde::Serialize;
use serde_json::Value;
use std::env::temp_dir;
use std::fs::{read_to_string, OpenOptions};
use std::io::{BufRead, Write};
use std::process;

/// Opens a JSON temp file in the user's preferred editor and on save and
/// close, runs a callback with the result.
///
/// The EDITOR environment variable stores the executable of the user's
/// preferred editor, which is called on a temp JSON file created in the
/// user's system temporary directory. When the user saves and exits,
/// if the file is still valid JSON, the callback `on_save` is called with
/// the new JSON object, otherwise an error is returned.
pub fn edit_as_json<T, F>(val: &T, mut on_save: F) -> Result<(), String>
where
    T: Serialize + ?Sized,
    F: FnMut(Value) -> Result<(), String>,
{
    // Create a temp file called `sitch.json`
    let mut temp_file_name = temp_dir();
    temp_file_name.push("sitch.json");
    let mut temp_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&temp_file_name)
        .map_err(|_err| {
            "Could not make a temporary file. Please make sure that the \
             current user has edit access to the system's config directory."
                .to_owned()
        })?;

    // Clear the file in case it already exists
    temp_file
        .set_len(0)
        .map_err(|_err| "Could not empty the temp file.".to_owned())?;
    let contents = format!("{}\n", serde_json::to_string_pretty(val).unwrap());
    // Save the input JSON object to the file
    temp_file.write_all(contents.as_bytes()).unwrap();
    // Edit the object in the user's preferred editor
    let editor = std::env::var("EDITOR").map_err(|_err| {
        "Could not find your preferred editor. Please set your \
         EDITOR environment variable when editing text."
            .to_owned()
    })?;
    process::Command::new(editor)
        .arg(&temp_file_name)
        .output()
        .map_err(|err| format!("An error occurred while editing the JSON object: {}", err))?;

    // if the edited JSON is still valid,
    let edited_json = read_to_string(&temp_file_name)
        .map_err(|_| "Could not read temp file after editing. Did it get deleted?".to_owned())?;
    let json = serde_json::from_str(&edited_json).map_err(|_| {
        "The edited object could not be parsed as JSON. Please try again.".to_owned()
    })?;

    //  run `on_save` on it
    on_save(json)
}

/// Reads input from stdin intelligently.
///
/// This will send a prompt to stdout and then await
/// some input. On input, if the provided value is either
/// "q" or "quit", the program exits. Otherwise, the input
/// (even an empty line) is passed to the `validate` callback
/// which either returns the parsed value or an error, which
/// is printed to stderr and then the prompt is asked again.
pub fn readline<T, F>(prompt: &str, mut validate: F) -> T
where
    F: FnMut(String) -> Result<T, String>,
{
    let stdin = std::io::stdin();
    loop {
        print!("{}", prompt);
        std::io::stdout().flush().unwrap();
        let input = stdin
            .lock()
            .lines()
            .next()
            .unwrap_or_else(|| {
                // If an interrupt is sent, makes sure that an
                // extra line is sent.
                println!("");
                std::process::exit(0);
            })
            .unwrap();
        // handle quitting
        if &input == "q" || &input == "quit" {
            std::process::exit(0);
        } else {
            match validate(input) {
                Ok(val) => return val,
                Err(err) => eprintln!("{}", err),
            }
        }
    }
}
