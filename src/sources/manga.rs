//! The Manga platform for update checking.

use crate::sources::{CheckForUpdates, SourceUpdate};
use crate::util::readline;
use chrono::{DateTime, Local, TimeZone};
use colored::Colorize;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The wrapper type for manga and their last checked times
/// to implement `CheckForUpdates` on.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MangaList(pub Vec<(Manga, Option<DateTime<Local>>)>);

// A manga source struct.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manga {
    pub name: String,
    pub id: String,
}

impl CheckForUpdates for MangaList {
    fn check_for_all_updates(
        &mut self,
        sitch_last_checked: &Option<DateTime<Local>>,
    ) -> Vec<(String, Result<Vec<SourceUpdate>, String>)> {
        self.0
            .par_iter_mut()
            .map(|(manga, last_checked)| {
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
                let update = manga.check_for_updates(&true_last_checked);
                // update last_checked if an update occurred
                if update.as_ref().map(|updates| updates.len()).unwrap_or(0) > 0 {
                    *last_checked = Some(Local::now());
                } else if last_checked.is_none() {
                    // if this source hasn't been checked yet, but no update was
                    // found, set it to the "global" `last_checked` time
                    *last_checked = sitch_last_checked.clone();
                }
                (manga.name.clone(), update)
            })
            .collect()
    }

    fn type_name(&self) -> &'static str {
        "Manga"
    }
}

impl Manga {
    pub fn check_for_updates(
        &self,
        last_checked: &Option<DateTime<Local>>,
    ) -> Result<Vec<SourceUpdate>, String> {
        // retrieve the API search data as JSON or return an error
        let query = format!("https://www.mangaeden.com/api/manga/{}/", self.id);
        let data: Value = reqwest::get(&query)
            .map_err(|_err| format!("Couldn't access {}", query))?
            .json()
            .map_err(|_err| "Couldn't parse request data as JSON".to_owned())?;

        // load specifically the chapter data from the returned JSON object
        let chapters = data
            .pointer("/chapters")
            .and_then(|chapters_obj| chapters_obj.as_array())
            .ok_or("Could not find chapters in received JSON")?;

        let base_chapter_url = data.pointer("/url").and_then(|url_obj| url_obj.as_str());

        // [
        //     41,                               - The chapter number
        //     1543389646.0,                     - The timestamp (epoch)
        //     "A Spiritually Transmitted Cold", - The chapter title
        //     "5bfe41ce719a167a5c3e2c98"        - The id (unused)
        // ],
        let mut recent_chapters = chapters
            .iter()
            .filter_map(|chapter_obj| {
                let chapter = chapter_obj.as_array()?;
                let published_date = chapter
                    .get(1)
                    .and_then(|timestamp_obj| timestamp_obj.as_f64())
                    .map(|timestamp| Local.timestamp(timestamp as i64, 0))
                    .filter(|pub_date| {
                        last_checked
                            .map(|last_checked| last_checked < *pub_date)
                            .unwrap_or(true)
                    })?;
                let chapter_number = chapter.get(0).and_then(|index_obj| index_obj.as_u64())?;
                let title = chapter
                    .get(2)
                    .and_then(|title_obj| title_obj.as_str())
                    .map(|title| format!("Chapter {} - {}", chapter_number, title))?;
                let link = base_chapter_url
                    .map(|url| format!("{}/{}", url, chapter_number))
                    .unwrap_or("<no link>".to_owned());

                Some(SourceUpdate {
                    title,
                    link,
                    published_date,
                })
            })
            .collect::<Vec<SourceUpdate>>();

        // sort the chapters as they aren't always returned in the right order
        recent_chapters.sort_by_key(|update| update.published_date.clone());

        Ok(recent_chapters)
    }

    /// Search interactively for new manga to add to sitch.
    ///
    /// Reads from stdin to take input and asks the user before any
    /// sources are added.
    pub fn interactive_search() -> Result<Self, String> {
        loop {
            // Take a query for input
            let search_term = readline("Search for an manga by name: ", |search| {
                if search.len() > 3 {
                    Ok(search)
                } else {
                    Err("Search term must be longer than 3 characters.".to_owned())
                }
            });

            // parse the query's returned data as JSON
            let query = "https://www.mangaeden.com/api/list/0/";
            let data: Value = reqwest::get(query)
                .map_err(|_err| format!("Couldn't access {}", query))?
                .json()
                .map_err(|_err| "Couldn't parse request data as JSON".to_owned())?;

            // format the results for the user to pick from
            let search_results = data
                .pointer("/manga")
                .and_then(|manga_obj| manga_obj.as_array())
                .ok_or("Couldn't parse received manga as JSON array".to_owned())?
                .iter()
                .map(|search_result| {
                    let id = search_result
                        .pointer("/i")
                        .and_then(|id_obj| id_obj.as_str())
                        .ok_or("No id found in search result".to_owned())?
                        .to_string();
                    let title = search_result
                        .pointer("/t")
                        .and_then(|title_obj| title_obj.as_str())
                        .ok_or("No title found for search result".to_owned())?
                        .to_owned();

                    Ok((title, id))
                })
                .filter(|opt_result| match opt_result {
                    Ok((title, _id)) => title.to_lowercase().contains(&search_term),
                    Err(_err) => true,
                })
                .take(5)
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
