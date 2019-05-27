//! The Anime platform for update checking.

use crate::sources::{CheckForUpdates, SourceUpdate};
use crate::util::readline;
use chrono::{DateTime, FixedOffset, Local};
use colored::Colorize;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The wrapper type for Bandcamp artists and their last checked times
/// to implement `CheckForUpdates` on.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AnimeList(pub Vec<(Anime, Option<DateTime<Local>>)>);

/// An anime source struct.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Anime {
    pub name: String,
    pub id: String,
}

impl CheckForUpdates for AnimeList {
    fn check_for_all_updates(
        &mut self,
        sitch_last_checked: &Option<DateTime<Local>>,
    ) -> Vec<(String, Result<Vec<SourceUpdate>, String>)> {
        self.0
            .par_iter_mut()
            .map(|(anime, last_checked)| {
                // use the earliest `last_checked` time provided either by sitch generally
                // or by this source to handle whe the user overrides the `last_checked` time
                let true_last_checked = if sitch_last_checked.is_some() && last_checked.is_some() {
                    Some(std::cmp::min(
                        sitch_last_checked.unwrap(),
                        last_checked.unwrap(),
                    ))
                } else {
                    last_checked.or(*sitch_last_checked)
                };
                let update = anime.check_for_updates(&true_last_checked);
                // update last_checked if an update occurred
                if update
                    .as_ref()
                    .map(|updates| updates.len() > 0)
                    .unwrap_or(false)
                {
                    *last_checked = Some(Local::now());
                } else if last_checked.is_none() {
                    // if this source hasn't been checked yet, but no update was
                    // found, set it to the "global" `last_checked` time
                    *last_checked = sitch_last_checked.clone();
                }
                (anime.name.clone(), update)
            })
            .collect()
    }

    fn type_name(&self) -> &'static str {
        "Anime"
    }
}

impl Anime {
    pub fn check_for_updates(
        &self,
        last_checked: &Option<DateTime<Local>>,
    ) -> Result<Vec<SourceUpdate>, String> {
        // retrieve the API search data as JSON or return an error
        let query = format!("https://api.jikan.moe/v3/anime/{}/episodes/1", self.id);
        let data: Value = reqwest::get(&query)
            .map_err(|_err| format!("Couldn't access {}", query))?
            .json()
            .map_err(|_err| "Couldn't parse request data as JSON".to_owned())?;

        //  retrieve the episode data from the JSON object
        let episodes = data
            .pointer("/episodes")
            .and_then(|episodes_obj| episodes_obj.as_array())
            .ok_or("Could not find episodes in received JSON")?;

        let mut recent_episodes = episodes
            .iter()
            .filter_map(|episode| {
                // parse the published date for each episode
                let published_date = episode
                    .pointer("/aired")
                    .and_then(|date_obj| date_obj.as_str())
                    .and_then(|date_str| DateTime::<FixedOffset>::parse_from_rfc3339(date_str).ok())
                    .map(|date| date.with_timezone(&Local))
                    // ignore episodes aired before last_checked if it was provided
                    .filter(|local_date| {
                        last_checked
                            .map(|last_checked| last_checked < *local_date)
                            .unwrap_or(true)
                    })?;
                // parse episode_id for ther title
                let episode_number = episode
                    .pointer("/episode_id")
                    .and_then(|id_obj| id_obj.as_u64())?;
                let title = format!(
                    "Episode {} - {}",
                    episode_number,
                    episode
                        .pointer("/title")
                        .and_then(|title_obj| title_obj.as_str())?
                );
                // parse the link for the update
                let link = episode
                    .pointer("/video_url")
                    .and_then(|link_obj| link_obj.as_str())?
                    .to_owned();

                Some(SourceUpdate {
                    title,
                    link,
                    published_date,
                })
            })
            .collect::<Vec<SourceUpdate>>();

        // sort the episodes by date as they aren't always
        // returned in sorted order by the API
        recent_episodes.sort_by_key(|update| update.published_date.clone());

        Ok(recent_episodes)
    }

    /// Search interactively for new anime to add to sitch.
    ///
    /// Reads from stdin to take input and asks the user before any
    /// sources are added.
    pub fn interactive_search() -> Result<Self, String> {
        loop {
            // Take a query for input
            let search_term = readline("Search for an anime by name: ", |search| {
                if search.len() > 3 {
                    Ok(search)
                } else {
                    Err("Search term must be longer than 3 characters.".to_owned())
                }
            });

            // parse the query's returned data as JSON
            let query = format!(
                "https://api.jikan.moe/v3/search/anime?q={}&limit=5",
                search_term
            );
            let data: Value = reqwest::get(&query)
                .map_err(|_err| format!("Couldn't access {}", query))?
                .json()
                .map_err(|_err| "Couldn't parse request data as JSON".to_owned())?;

            // format the results for the user to pick from
            let search_results = data
                .pointer("/results")
                .and_then(|results_obj| results_obj.as_array())
                .ok_or("Couldn't parse results as JSON array".to_owned())?
                .iter()
                .map(|search_result| {
                    let id = search_result
                        .pointer("/mal_id")
                        .and_then(|id_obj| id_obj.as_u64())
                        .ok_or("No id found in search result".to_owned())?
                        .to_string();
                    let title = search_result
                        .pointer("/title")
                        .and_then(|title_obj| title_obj.as_str())
                        .ok_or("No title found for search result".to_owned())?
                        .to_owned();

                    Ok((title, id))
                })
                .collect::<Result<Vec<(String, String)>, String>>()?;

            match search_results.len() {
                // try again if there were no results found
                0 => println!("No results found, please try again."),
                1 => {
                    // if only one was found, ask if they want to add it.
                    // if they don't, exit from sitch.
                    let (title, id) = search_results.into_iter().next().unwrap();
                    println!("Found 1 result: \"{}\" (id = {})", title, id);
                    let should_add =
                        readline("Add it to sitch? [Y/n]", |input| match input.as_str() {
                            "" | "y" | "Y" | "yes" => Ok(true),
                            "n" | "N" | "no" => Ok(false),
                            _ => Err("Please respond with a yes or no.".to_owned()),
                        });
                    if should_add {
                        return Ok(Self { name: title, id });
                    } else {
                        std::process::exit(0);
                    }
                }
                num_results => {
                    // if multiple were found, print how many were found and then
                    // enumerate them. Let the user choose one of them to add to sitch.
                    println!("Found {} results:", num_results);
                    for (index, (title, id)) in search_results.iter().enumerate() {
                        println!(
                            "{}: \"{}\" (id = {})",
                            (index + 1).to_string().yellow(),
                            title.green(),
                            id
                        );
                    }
                    let index = readline(
                        &format!("Pick a result to add [1 to {}]: ", num_results),
                        |picked| match picked.parse::<usize>() {
                            Ok(index) if (1 <= index && index <= num_results) => Ok(index - 1),
                            Ok(_bad_index) => {
                                Err("The specified index was out of bounds.".to_owned())
                            }
                            Err(_err) => Err("The value wasn't an integer.".to_owned()),
                        },
                    );
                    let (name, id) = search_results.into_iter().nth(index).unwrap();
                    return Ok(Self { name, id });
                }
            }
        }
    }
}
