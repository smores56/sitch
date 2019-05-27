//! The YouTube platform for update checking.

use crate::sources::{CheckForUpdates, SourceUpdate};
use crate::util::readline;
use chrono::{DateTime, FixedOffset, Local};
use colored::Colorize;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The wrapper type for YouTube channels and their last checked times
/// to implement `CheckForUpdates` on.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct YouTubeChannels {
    pub api_key: Option<String>,
    pub channels: Vec<(YouTubeChannel, Option<DateTime<Local>>)>,
}

/// A YouTube channel struct.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct YouTubeChannel {
    pub name: String,
    pub channel_id: String,
}

impl CheckForUpdates for YouTubeChannels {
    fn check_for_all_updates(
        &mut self,
        sitch_last_checked: &Option<DateTime<Local>>,
    ) -> Vec<(String, Result<Vec<SourceUpdate>, String>)> {
        // only check for updates if an API key is provided
        if let Some(api_key) = &self.api_key {
            self.channels
                .par_iter_mut()
                .map(|(channel, last_checked)| {
                    // use the earliest `last_checked` time provided either by sitch generally
                    // or by this source to handle whe the user overrides the `last_checked` time
                    let true_last_checked =
                        if sitch_last_checked.is_some() && last_checked.is_some() {
                            Some(std::cmp::min(
                                sitch_last_checked.unwrap(),
                                last_checked.unwrap(),
                            ))
                        } else {
                            last_checked.or(*sitch_last_checked)
                        };
                    let update = channel.check_for_updates(api_key, &true_last_checked);
                    // update last_checked if an update occurred
                    if update.as_ref().map(|updates| updates.len()).unwrap_or(0) > 0 {
                        *last_checked = Some(Local::now());
                    } else if last_checked.is_none() {
                        // if this source hasn't been checked yet, but no update was
                        // found, set it to the "global" `last_checked` time
                        *last_checked = sitch_last_checked.clone();
                    }
                    (channel.name.clone(), update)
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    fn type_name(&self) -> &'static str {
        "YouTube"
    }
}

impl YouTubeChannel {
    pub fn check_for_updates(
        &self,
        api_key: &str,
        last_checked: &Option<DateTime<Local>>,
    ) -> Result<Vec<SourceUpdate>, String> {
        // query YouTube's v3 API for videos from the given channel
        let base_url = "https://www.googleapis.com/youtube/v3/search";
        let published_after = last_checked
            .map(|date| date.to_rfc3339())
            .unwrap_or("1970-01-01T00:00:00Z".to_owned());
        let params = vec![
            ("part", "snippet"),
            ("channelId", &self.channel_id),
            ("maxResults", "25"),
            ("order", "date"),
            ("type", "video"),
            ("key", api_key),
            ("publishedAfter", &published_after),
        ];
        let query = format!(
            "{}?{}",
            base_url,
            params
                .into_iter()
                .map(|(key, value)| format!("{}={}", key, value))
                .collect::<Vec<_>>()
                .join("&")
        );

        // retrieve the API search data as JSON
        let data: Value = reqwest::get(&query)
            .map_err(|_err| format!("Couldn't access {}", query))?
            .json()
            .map_err(|_err| "Couldn't parse request data as JSON".to_owned())?;

        let items: &Vec<Value> = data
            .pointer("/items")
            .and_then(|obj| obj.as_array())
            .ok_or("YouTube API JSON data wasn't an object")?;

        Ok(items
            .into_iter()
            .filter_map(|item| {
                // parse the published_date
                let pub_date_str = item
                    .pointer("/snippet/publishedAt")
                    .and_then(|date_obj| date_obj.as_str())?;
                let published_date = DateTime::<FixedOffset>::parse_from_rfc3339(pub_date_str)
                    .map(|date| date.with_timezone(&Local))
                    .ok()?;
                // parse the title of the video
                let title = item
                    .pointer("/snippet/title")
                    .and_then(|title_obj| title_obj.as_str())
                    .map(|title| title)
                    .unwrap_or("<unnamed>")
                    .to_owned();
                // parse the link to the video
                let link = item
                    .pointer("/id/videoId")
                    .and_then(|id_obj| id_obj.as_str())
                    .map(|id| format!("https://www.youtube.com/watch?v={}", id))
                    .unwrap_or("<no link>".to_owned());

                Some(SourceUpdate {
                    title,
                    link,
                    published_date,
                })
            })
            .collect())
    }
}

impl YouTubeChannels {
    /// Search interactively for new YouTube channels to add to sitch.
    ///
    /// Reads from stdin to take input and asks the user before any
    /// channels are added.
    pub fn interactive_search(&self) -> Result<YouTubeChannel, String> {
        // only run if an API key is provided
        if self.api_key.is_none() {
            return Err("Must have API key set to search for YouTube channels.".to_owned());
        }

        loop {
            // Take a query for input
            let search_term = readline("Search for an channel by name: ", |search| {
                if search.len() > 3 {
                    Ok(search)
                } else {
                    Err("Search term must be longer than 3 characters.".to_owned())
                }
            });

            // query YouTube's v3 API for relevant channels
            let api_key = self.api_key.clone().unwrap();
            let base_url = "https://content.googleapis.com/youtube/v3/search";
            let params = vec![
                ("part", "snippet"),
                ("maxResults", "5"),
                ("type", "channel"),
                ("key", &api_key),
                ("q", &search_term),
            ];
            let query = format!(
                "{}?{}",
                base_url,
                params
                    .into_iter()
                    .map(|(key, value)| format!("{}={}", key, value))
                    .collect::<Vec<_>>()
                    .join("&")
            );

            // parse the query's returned data as JSON
            let data: Value = reqwest::get(&query)
                .map_err(|_err| format!("Couldn't access {}", query))?
                .json()
                .map_err(|_err| "Couldn't parse request data as JSON".to_owned())?;

            // {
            //     ...
            //     "items": [
            //         {
            //             "snippet": {
            //                 "channelId": "UC9XtgFNeoDbjISzoJT0Qi9w",
            //                 "channelTitle": "Shnabubula",
            //                 ...
            //             },
            //             ...
            //         },
            //         ...
            //     ]
            // format the results for the user to pick from
            let search_results = data
                .pointer("/items")
                .and_then(|results_obj| results_obj.as_array())
                .ok_or("Couldn't parse results as JSON array".to_owned())?
                .iter()
                .map(|search_result| {
                    let channel_id = search_result
                        .pointer("/snippet/channelId")
                        .and_then(|id_obj| id_obj.as_str())
                        .ok_or("No id found in search result".to_owned())?
                        .to_owned();
                    let channel_name = search_result
                        .pointer("/snippet/channelTitle")
                        .and_then(|title_obj| title_obj.as_str())
                        .ok_or("No title found for search result".to_owned())?
                        .to_owned();

                    Ok((channel_id, channel_name))
                })
                .collect::<Result<Vec<(String, String)>, String>>()?;

            match search_results.len() {
                // try again if there were no results found
                0 => println!("No results found, please try again."),
                1 => {
                    // if only one was found, ask if they want to add it.
                    // if they don't, exit from sitch.
                    let (channel_id, name) = search_results.into_iter().next().unwrap();
                    println!("Found 1 result: \"{}\" (id = {})", name, channel_id);
                    let should_add =
                        readline("Add it to sitch? [Y/n]", |input| match input.as_str() {
                            "" | "y" | "Y" | "yes" => Ok(true),
                            "n" | "N" | "no" => Ok(false),
                            _ => Err("Please respond with a yes or no.".to_owned()),
                        });
                    if should_add {
                        return Ok(YouTubeChannel { name, channel_id });
                    } else {
                        std::process::exit(0);
                    }
                }
                num_results => {
                    // if multiple were found, print how many were found and then
                    // enumerate them. Let the user choose one of them to add to sitch.
                    println!("Found {} results:", num_results);
                    for (index, (channel_id, name)) in search_results.iter().enumerate() {
                        println!(
                            "{}: \"{}\" (id = {})",
                            (index + 1).to_string().yellow(),
                            name.green(),
                            channel_id
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
                    let (channel_id, name) = search_results.into_iter().nth(index).unwrap();
                    return Ok(YouTubeChannel { name, channel_id });
                }
            }
        }
    }
}
