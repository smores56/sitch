//! Handles checking for updates on different
//! platforms and rporting them to the user.

pub mod anime;
pub mod bandcamp;
pub mod manga;
pub mod rss;
pub mod youtube;

use self::rss::RssSources;
use anime::AnimeList;
use atty::Stream;
use bandcamp::BandcampArtists;
use chrono::{DateTime, Local};
use colored::Colorize;
use dirs::config_dir;
use manga::MangaList;
use notify_rust::Notification;
use rayon::iter::{IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::{Borrow, BorrowMut};
use std::fs::{read_to_string, write, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use youtube::YouTubeChannels;

/// The struct used for configuration. Holds the time sitch last
/// found an update for one of its sources as well as the config
/// info for each platform individually.
#[derive(Serialize, Deserialize, Default)]
pub struct Sources {
    pub last_checked: Option<DateTime<Local>>,
    pub rss: RssSources,
    pub youtube: YouTubeChannels,
    pub anime: AnimeList,
    pub manga: MangaList,
    pub bandcamp: BandcampArtists,
}

impl Sources {
    /// Attempts to load the config data from a JSON file.
    ///
    /// Either the data is located in a JSON file at a specified path
    /// given by `config_path` or from a default file `$CONFIG_DIR/sitch/config.json`.
    /// Each individual source is deserialized separately to allow for source
    /// files to continue to work if new source platforms are added to sitch
    /// in later versions.
    pub fn load(config_path: Option<PathBuf>) -> Result<Self, String> {
        let json = Self::load_config(config_path)?;

        Ok(Sources {
            last_checked: Self::parse_from_config(&json, "last_checked")?,
            rss: Self::parse_from_config(&json, "rss")?,
            youtube: Self::parse_from_config(&json, "youtube")?,
            anime: Self::parse_from_config(&json, "anime")?,
            manga: Self::parse_from_config(&json, "manga")?,
            bandcamp: Self::parse_from_config(&json, "bandcamp")?,
        })
    }

    /// Attempts to parse a field from a JSON (Value) object.
    ///
    /// If there is an object in the JSON where the pointer specifies, this
    /// function attempts to parse it: if the parsing fails, an error is returned.
    /// If no object is found, however, then the default of the specified type to
    /// deserialize is generated.
    fn parse_from_config<'de, T>(config: &'de Value, field: &str) -> Result<T, String>
    where
        T: Deserialize<'de> + Sized + Default,
    {
        if let Some(field_obj) = config.pointer(&format!("/{}", field)) {
            T::deserialize(field_obj)
                .map_err(|err| format!("Couldn't parse {} from config.json: {}", field, err))
        } else {
            Ok(T::default())
        }
    }

    /// Attempts to load the contents of the JSON config file.
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

    /// Determines the config path for sitch to use.
    ///
    /// If one is provided, that is used. If not, the system's config directory
    /// is searched for. A directory named `sitch` is added to it, and the new
    /// path `$CONFIG_DIR/sitch/config.json` is returned.
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

    /// Checks for updates from the currently configured sources.
    ///
    /// * `quiet` - whether to simplify the output and suppress errors.
    /// * `notify` - whether to output updates and errors as notifications.
    ///              Nothing is printed, and this overrides `quiet`.
    ///
    /// This relies heavily on rayon for parallelization to speed up the
    /// runtime of sitch. Not only are all source platforms checked in parallel,
    /// but also are each of the specific sources in each platform are
    /// checked in parallel, too.
    pub fn check_for_updates(&mut self, quiet: bool, notify: bool) {
        let last_checked = self.last_checked.clone();
        // put all platforms into a vec for easy parallelization
        let mut sources: Vec<Box<&mut CheckForUpdates>> = vec![
            Box::new(&mut self.rss),
            Box::new(&mut self.youtube),
            Box::new(&mut self.anime),
            Box::new(&mut self.manga),
            Box::new(&mut self.bandcamp),
        ];

        // used to determine whether to update last_checked
        let update_occurred = Arc::new(Mutex::new(false));
        // used for making sure that clicking notifications to open
        // links works by waiting for each notification thread
        let notification_threads = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));
        // used to give a runtime for each source update
        let before = Instant::now();
        sources
            .par_iter_mut()
            .flat_map(|source| {
                source
                    .check_for_all_updates(&last_checked)
                    .into_par_iter()
                    .map(move |(source_name, result)| (source.type_name(), source_name, result))
            })
            .for_each(
                |(type_name, source_name, update_result)| match update_result {
                    Ok(all_updates) => {
                        // if any updates occurred,
                        if all_updates.len() > 0 {
                            if !*(update_occurred.lock().unwrap()) {
                                // if running in normal mode, print a preamble that
                                // updates have occurred
                                if !quiet && !notify {
                                    if let Some(last_checked) = last_checked {
                                        println!(
                                            "The following sources have updated since {}:",
                                            last_checked.format("%B %d, %Y at %-l:%M %p")
                                        );
                                    } else {
                                        println!("The following sources have updates:");
                                    }
                                }
                                **(update_occurred.lock().unwrap().borrow_mut()) = true;
                            }
                            let seconds = before.elapsed().as_secs();
                            if notify {
                                // spawn a notification that waits until it is dismissed
                                // or the relevant update is clicked
                                let update = all_updates[0].clone();
                                notification_threads.lock().unwrap().borrow_mut().push(
                                    thread::spawn(move || {
                                        Notification::new()
                                            .summary(&format!("Sitch - {}", source_name))
                                            .body(&update.title)
                                            .action("open", "Open in Browser")
                                            .timeout(0)
                                            .show()
                                            .unwrap()
                                            .wait_for_action(|action| {
                                                if action == "open" {
                                                    webbrowser::open(&update.link).ok();
                                                }
                                            });
                                    }),
                                );
                            } else if quiet {
                                // simplify output if in quiet mode
                                let update = &all_updates[0];
                                // handle piping vs. printing to a terminal correctly
                                if atty::is(Stream::Stdout) {
                                    println!(
                                        "{}: \"{}\" {}",
                                        source_name.green(),
                                        update.title,
                                        update.link.bright_blue(),
                                    );
                                } else {
                                    println!(
                                        "{}: \"{}\" {}",
                                        source_name, update.title, update.link,
                                    );
                                }
                            } else {
                                // otherwise print in normal, verbose mode
                                // handle piping vs. printing to a terminal correctly
                                if atty::is(Stream::Stdout) {
                                    println!(
                                        "{} - {}: {} {}",
                                        type_name.green(),
                                        source_name.green(),
                                        SourceUpdate::message(&all_updates, true),
                                        format!(
                                            "[{} second{}]",
                                            seconds,
                                            if seconds != 1 { "s" } else { "" }
                                        )
                                        .purple()
                                    );
                                } else {
                                    println!(
                                        "{} - {}: {} [{} second{}]",
                                        type_name,
                                        source_name,
                                        SourceUpdate::message(&all_updates, false),
                                        seconds,
                                        if seconds != 1 { "s" } else { "" }
                                    );
                                }
                            }
                        }
                    }
                    Err(error) => {
                        // only care about errors if in normal or notification mode
                        if notify {
                            // if in notification mode, don't need to wait until all
                            // updates are reported to report errors, so the notification
                            // can be displayed immediately for errors
                            Notification::new()
                                .summary(&format!("Sitch Error - {}", source_name))
                                .body(&error)
                                .show()
                                .unwrap();
                        } else if !quiet {
                            // if in normal mode, though, add to a list of errors
                            // reporting errors after all updates have been displayed
                            errors.lock().unwrap().borrow_mut().push((
                                type_name,
                                source_name,
                                error,
                                before.elapsed().as_secs(),
                            ));
                        }
                    }
                },
            );

        if *(update_occurred.lock().unwrap()) {
            // if an update occurred, update the last checked time for
            // sitch to know about on the next run
            self.last_checked = Some(Local::now());
        } else if !quiet && !notify {
            // only in normal mode does sitch print this message
            eprintln!("No updates at this time.");
        }

        if errors.lock().unwrap().len() > 0 {
            // if there are errors (which are only added to the list of
            // errors in normal mode), then report them here
            eprintln!("\nThe following errors occurred:");
            for (type_name, source_name, error, secs) in errors.lock().unwrap().borrow().iter() {
                // handle piping vs. printing to a terminal
                if atty::is(atty::Stream::Stderr) {
                    eprintln!(
                        "{} - {}: {} {}",
                        type_name.red(),
                        source_name.red(),
                        error,
                        format!("[{} second{}]", secs, if *secs != 1 { "s" } else { "" }).purple()
                    );
                } else {
                    eprintln!(
                        "{} - {}: {} [{} second{}]",
                        type_name,
                        source_name,
                        error,
                        secs,
                        if *secs != 1 { "s" } else { "" }
                    );
                }
            }
        }

        // if any notifications that can be clicked on were displayed,
        // wait for them to either be clicked or dismissed here
        for handle in Arc::try_unwrap(notification_threads)
            .unwrap()
            .into_inner()
            .unwrap()
        {
            handle.join().unwrap();
        }
    }

    /// Save the config info as JSON into the config file determined
    /// by both the optional `config_path` argument.
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

/// A trait for all platforms that can check for updates to implement.
///
/// All implementors must be `Send` + `Sync` in order to work with
/// rayon's parallelization.
pub trait CheckForUpdates: Send + Sync {
    /// Check for all source updates on a platform.
    ///
    /// Updates each source's last_checked time for each that receives
    /// an update. Returns a list of tuples, with each tuple holding
    /// the name of the source and a result holding either a list of
    /// updates or an error message that occurred while checking for
    /// updates.
    fn check_for_all_updates(
        &mut self,
        last_checked: &Option<DateTime<Local>>,
    ) -> Vec<(String, Result<Vec<SourceUpdate>, String>)>;

    /// The name of the platform (aka "YouTube").
    ///
    /// This is a method on each struct rather than an associated
    /// method due to the limits of the type system at the time
    /// of writing sitch.
    fn type_name(&self) -> &'static str;
}

/// An update from a source.
#[derive(Clone)]
pub struct SourceUpdate {
    /// The title of the update.
    pub title: String,
    /// An absolute link to the update.
    pub link: String,
    /// When the update was published.
    pub published_date: DateTime<Local>,
}

impl SourceUpdate {
    /// Prints the most recent update from the given
    /// list of updates (assumed to be the first one).
    ///
    /// *tty* - Colors output if printing to a terminal.
    ///
    /// The output format if there is only one update is generally:
    /// "There has been 1 update, it was \"<update title>\" released
    ///  on <published date>, found here: <update link>"
    ///
    /// The output format if there have been multiple updates is generally:
    /// "There have been X updates, the earliest was \"<update title>\"
    ///  released on <published date>, found here: <update link>"
    ///
    /// # Panics:
    /// This method will panic if it is given an
    /// empty list of updates.
    pub fn message(updates: &Vec<Self>, tty: bool) -> String {
        let number_of_updates = updates.len();
        // make sure that there is at least one update
        assert!(number_of_updates > 0);
        let update = &updates[0];

        let datetime_format = "%B %-e, %Y at %-l:%M %p";
        let number_of_updates_str = if number_of_updates == 1 {
            "has been 1 update".to_owned()
        } else {
            format!("have been {} updates", number_of_updates)
        };
        let update_str = if tty {
            format!(
                "\"{}\" released on {}, found here: {}",
                update.title,
                update.published_date.format(datetime_format),
                update.link.bright_blue()
            )
        } else {
            format!(
                "\"{}\" released on {}, found here: {}",
                update.title,
                update.published_date.format(datetime_format),
                update.link
            )
        };

        format!(
            "There {}, {} was {}",
            number_of_updates_str,
            if number_of_updates == 1 {
                "it"
            } else {
                "the earliest"
            },
            update_str,
        )
    }
}
