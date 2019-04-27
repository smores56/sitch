use chrono::{DateTime, FixedOffset, Local};
use colored::Colorize;
use rss::{Channel, Item};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RssSource {
    pub name: String,
    pub feed: String,
}

impl RssSource {
    pub fn check_for_update(
        &self,
        last_checked: &Option<DateTime<Local>>,
    ) -> Result<Option<String>, String> {
        let channel = Channel::from_url(&self.feed)
            .map_err(|err| format!("Couldn't load RSS feed from {}: {}", self.feed, err))?;
        let items = channel.into_items();
        let mut updates = items
            .into_iter()
            .filter_map(|item| {
                if let Ok(pub_date) =
                    DateTime::<FixedOffset>::parse_from_rfc2822(item.pub_date().unwrap_or(""))
                {
                    Some((item, pub_date.with_timezone(&Local))).filter(|(_item, pub_date)| {
                        last_checked
                            .map(|last_checked| &last_checked < pub_date)
                            .unwrap_or(true)
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<(Item, DateTime<Local>)>>();
        updates.sort_by_key(|(_item, pub_date)| pub_date.clone());
        let num_updates = updates.len();

        Ok(updates
            .into_iter()
            .map(|(item, pub_date)| {
                let datetime_format = "%B %-e, %Y at %-l:%M %p";
                let num_updates_str = if num_updates == 1 {
                    "has been 1 update".to_owned()
                } else {
                    format!("have been {} updates", num_updates)
                };
                let update_str = format!(
                    "\"{}\" released on {}, found here: {}",
                    item.title().unwrap_or("<unnamed>"),
                    pub_date.format(datetime_format),
                    item.link().unwrap_or("<no link>").bright_blue()
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
            })
            .next())
    }
}
