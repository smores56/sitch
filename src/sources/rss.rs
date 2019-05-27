//! The RSS feed platform for update checking.

use crate::sources::{CheckForUpdates, SourceUpdate};
use chrono::{DateTime, FixedOffset, Local};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use rss::Channel;
use serde::{Deserialize, Serialize};

/// The wrapper type for RSS feeds and their last checked times
/// to implement `CheckForUpdates` on.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RssSources(pub Vec<(RssSource, Option<DateTime<Local>>)>);

/// An RSS feed struct.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RssSource {
    pub name: String,
    pub feed: String,
}

impl CheckForUpdates for RssSources {
    fn check_for_all_updates(
        &mut self,
        sitch_last_checked: &Option<DateTime<Local>>,
    ) -> Vec<(String, Result<Vec<SourceUpdate>, String>)> {
        self.0
            .par_iter_mut()
            .map(|(rss, last_checked)| {
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
                let update = rss.check_for_updates(&true_last_checked);
                // update last_checked if an update occurred
                if update.as_ref().map(|updates| updates.len()).unwrap_or(0) > 0 {
                    *last_checked = Some(Local::now());
                } else if last_checked.is_none() {
                    // if this source hasn't been checked yet, but no update was
                    // found, set it to the "global" `last_checked` time
                    *last_checked = sitch_last_checked.clone();
                }
                (rss.name.clone(), update)
            })
            .collect()
    }

    fn type_name(&self) -> &'static str {
        "RSS"
    }
}

impl RssSource {
    pub fn check_for_updates(
        &self,
        last_checked: &Option<DateTime<Local>>,
    ) -> Result<Vec<SourceUpdate>, String> {
        // load the RSS feed items or return an error
        let channel = Channel::from_url(&self.feed)
            .map_err(|err| format!("Couldn't load RSS feed from {}: {}", self.feed, err))?;
        let items = channel.into_items();

        Ok(items
            .into_iter()
            .filter_map(|item| {
                // parse the feed items and determine which items were published
                // after the last_checked date if it was provided
                DateTime::<FixedOffset>::parse_from_rfc2822(item.pub_date().unwrap_or(""))
                    .ok()
                    .map(|pub_date| (item, pub_date.with_timezone(&Local)))
                    .filter(|(_item, pub_date)| {
                        last_checked
                            .map(|last_checked| &last_checked < pub_date)
                            .unwrap_or(true)
                    })
            })
            .map(|(item, published_date)| SourceUpdate {
                title: item.title().unwrap_or("<unnamed>").to_owned(),
                link: item.link().unwrap_or("<no link>").to_owned(),
                published_date,
            })
            .collect())
    }
}
