//! The Bandcamp platform for update checking.

use crate::sources::{CheckForUpdates, SourceUpdate};
use chrono::{DateTime, Local, TimeZone};
use rayon::iter::{IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use select::document::Document;
use select::predicate::{Attr, Class, Name, Predicate};
use serde::{Deserialize, Serialize};

/// The wrapper type for Bandcamp artists and their last checked times
/// to implement `CheckForUpdates` on.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct BandcampArtists(pub Vec<(BandcampArtist, Option<DateTime<Local>>)>);

/// A Bandcamp artist struct.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BandcampArtist {
    pub name: String,
    pub url: String,
}

impl CheckForUpdates for BandcampArtists {
    fn check_for_all_updates(
        &mut self,
        sitch_last_checked: &Option<DateTime<Local>>,
    ) -> Vec<(String, Result<Vec<SourceUpdate>, String>)> {
        self.0
            .par_iter_mut()
            .map(|(artist, last_checked)| {
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
                let update = artist.check_for_updates(&true_last_checked);
                // update last_checked if an update occurred
                if update.as_ref().map(|updates| updates.len()).unwrap_or(0) > 0 {
                    *last_checked = Some(Local::now());
                } else if last_checked.is_none() {
                    // if this source hasn't been checked yet, but no update was
                    // found, set it to the "global" `last_checked` time
                    *last_checked = sitch_last_checked.clone();
                }
                (artist.name.clone(), update)
            })
            .collect()
    }

    fn type_name(&self) -> &'static str {
        "Bandcamp"
    }
}

impl BandcampArtist {
    /// Check for updates for a BandCamp artist.
    ///
    /// Quite unfortunately, Bandcamp disabled their general purpose
    /// API for exactly what sitch would need for all new users, only
    /// an API for an artist's own albums is available. Thus, we need
    /// to web-scrape to find updates for artists.
    pub fn check_for_updates(
        &self,
        last_checked: &Option<DateTime<Local>>,
    ) -> Result<Vec<SourceUpdate>, String> {
        // get the artist page and parse it as an HTML document
        let artist_page = reqwest::get(&self.url)
            .map_err(|err| format!("Could not fetch artist page: {}", err))?
            .text()
            .map_err(|_err| "No html found on artist page".to_owned())?;
        let artist_document = Document::from(artist_page.as_str());

        // <li class="music-grid-item square first-four">
        //     <a href="/album/meat-machine-ep"></a>
        // </li>
        // try the first type of artist page parsing to get album links
        let mut recent_album_links = artist_document
            .find(Name("li").and(Class("music-grid-item")))
            .filter_map(|node| {
                node.find(Name("a"))
                    .next()
                    .and_then(|link_el| link_el.attr("href"))
                    .map(|album_link| format!("{}{}", self.url, album_link))
            })
            // only take 10 max to minimize the number of requests made
            .take(10)
            .collect::<Vec<String>>();

        // if no links are found, try parsing the second type of pages
        if recent_album_links.len() == 0 {
            recent_album_links = artist_document
                .find(Name("div").and(Attr("id", "discography").descendant(Class("trackTitle"))))
                .filter_map(|node| {
                    node.find(Name("a"))
                        .next()
                        .and_then(|link_el| link_el.attr("href"))
                        .map(|album_link| format!("{}{}", self.url, album_link))
                })
                // only take 10 max to minimize the number of requests made
                .take(10)
                .collect::<Vec<String>>();
        }

        // in parallel, attempt to retrieve, parse, and then filter out
        // the first 10 albums on an artist's page to find updates
        recent_album_links
            .into_par_iter()
            .filter_map(|link| {
                // either load the page or return an error
                let mut album_page = match reqwest::get(&link) {
                    Ok(page) => page,
                    Err(err) => return Some(Err(format!("Could not fetch album page: {}", err))),
                };
                // either parse the page into HTML or return an error
                let album_document = match album_page.text() {
                    Ok(text) => Document::from(text.as_str()),
                    Err(_err) => return Some(Err("No html found on album page".to_owned())),
                };

                // parse the album name from the `class="trackTitle"` element
                let album_name = album_document
                    .find(Class("trackTitle"))
                    .next()
                    .map(|name_el| name_el.text().trim().to_owned())
                    .unwrap_or("<no album name>".to_owned());
                // parse the artist name from the `itemprop="byArtist"` element
                let artist = album_document
                    .find(Attr("itemprop", "byArtist").descendant(Name("a")))
                    .next()
                    .map(|artist_el| artist_el.text())
                    .unwrap_or("<no artist>".to_owned());
                // parse the published date from the below element, and
                // return an error if the parsing fails
                // <meta itemprop="datePublished" content="20190426">
                let published_date = match album_document
                    .find(Attr("itemprop", "datePublished"))
                    .next()
                    .and_then(|date_el| date_el.attr("content"))
                    .and_then(|date_str| {
                        Local
                            .datetime_from_str(&(date_str.to_owned() + "00:00:00"), "%Y%m%d%T")
                            .ok()
                    }) {
                    Some(date) => date,
                    None => return Some(Err(format!("No published date on album at {}", link))),
                };

                // only return albums published after the last_checked date if it is given
                Some(Ok(SourceUpdate {
                    title: format!("{} by {}", album_name, artist),
                    link,
                    published_date: Some(published_date).filter(|&date| {
                        last_checked.map(|checked| checked < date).unwrap_or(true)
                    })?,
                }))
            })
            .collect()
    }
}
