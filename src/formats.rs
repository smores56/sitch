use crate::rss_source::RssSource;
use crate::youtube::YouTubeChannels;
use chrono::{DateTime, Local, Utc};
use colored::Colorize;
use dirs::config_dir;
use serde::Deserialize;
use serde_json::Value;
use std::fs::{read_to_string, write, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(serde_derive::Serialize)]
pub struct Formats {
    pub last_checked: Option<DateTime<Local>>,
    pub rss: Vec<RssSource>,
    pub youtube: YouTubeChannels,
}

impl Formats {
    pub fn load(config_path: Option<PathBuf>) -> Result<Self, String> {
        let json = Self::load_config(config_path)?;
        let last_checked =
            if let Some(obj) = json.pointer("/last_checked") {
                Some(DateTime::<Local>::deserialize(obj).map_err(|err| {
                    format!("Couldn't parse last_checked in config.json: {}", err)
                })?)
            } else {
                None
            };
        let rss = match json.pointer("/rss") {
            Some(rss_obj) => Vec::<RssSource>::deserialize(rss_obj)
                .map_err(|err| format!("Couldn't parse rss in config.json: {}", err))?,
            None => Vec::new(),
        };
        let youtube = match json.pointer("/youtube") {
            Some(rss_obj) => YouTubeChannels::deserialize(rss_obj)
                .map_err(|err| format!("Couldn't parse youtube in config.json: {}", err))?,
            None => YouTubeChannels::default(),
        };

        Ok(Formats {
            last_checked,
            rss,
            youtube,
        })
    }

    fn load_config(config_path: Option<PathBuf>) -> Result<Value, String> {
        let path = Self::config_path(config_path)?;
        let contents = read_to_string(&path).or_else(|_| match write(&path, b"{}") {
            Ok(_) => Ok("{}".to_owned()),
            Err(_) => Err(format!(
                "Couldn't write to config file at {}.",
                path.to_string_lossy()
            )),
        })?;

        serde_json::from_str(&contents).map_err(|_| {
            format!(
                "Couldn't parse config contents. Please check that the config \
                 file at {} is properly formatted JSON.",
                path.to_string_lossy()
            )
        })
    }

    fn config_path(config_path: Option<PathBuf>) -> Result<PathBuf, String> {
        config_path
            .or_else(|| {
                config_dir().map(|dir| {
                    std::fs::create_dir(dir.join("sitch")).ok();
                    dir.join("sitch/config.json")
                })
            })
            .ok_or(
                "Could not find your system's config directory. \
                 Please specify a location for your config file."
                    .to_string(),
            )
    }

    pub fn check_for_updates(&mut self) {
        let (updates, errors) = {
            let mut updates = Vec::new();
            let mut errors = Vec::new();

            for (source, update_result) in self
                .rss
                .iter()
                .map(|source| (source, source.check_for_update(&self.last_checked)))
            {
                match update_result {
                    Ok(update) => {
                        if let Some(update) = update {
                            updates.push((source.name.as_str(), update));
                        }
                    }
                    Err(error) => errors.push((source.name.as_str(), error)),
                }
            }

            let (yt_updates, yt_errors) = self.youtube.check_for_updates(&self.last_checked);
            updates.extend(yt_updates.into_iter());
            errors.extend(yt_errors.into_iter());

            (updates, errors)
        };

        if updates.len() > 0 {
            if let Some(last_checked) = self.last_checked {
                println!(
                    "The following sources have updated since {}:",
                    last_checked.format("%B %d, %Y at %-l:%M %p")
                );
            } else {
                println!("The following sources have updates:");
            }

            for (source, update) in updates {
                println!("  {}: {}", source.green(), update);
            }

            self.last_checked = Some(Utc::now().with_timezone(&Local));
        } else {
            eprintln!("No updates at this time.");
        }

        if errors.len() > 0 {
            eprintln!("\nThe following errors occurred:");
            for (source, error) in errors {
                eprintln!("  {}: {}", source.red(), error);
            }
        }
    }

    pub fn save(&self, config_path: Option<PathBuf>) -> Result<(), String> {
        let path = Self::config_path(config_path)?;
        let file_data = serde_json::to_string_pretty(&self).unwrap();
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&path)
            .map_err(|_| {
                format!(
                    "Could not write to config.json file at {}.",
                    path.to_string_lossy()
                )
            })?;
        file.set_len(0).unwrap();
        file.write_all(format!("{}\n", file_data).as_bytes())
            .unwrap();

        Ok(())
    }
}
