use crate::util::update_message;
use chrono::{DateTime, FixedOffset, Local};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct YouTubeChannels {
    pub api_key: Option<String>,
    pub channels: Vec<YouTubeChannel>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct YouTubeChannel {
    pub name: String,
    pub channel_id: String,
}

impl YouTubeChannels {
    pub fn check_for_updates(
        &self,
        last_checked: &Option<DateTime<Local>>,
    ) -> (Vec<(&str, String)>, Vec<(&str, String)>) {
        let mut updates = Vec::new();
        let mut errors = Vec::new();

        if let Some(api_key) = &self.api_key {
            for (channel, update_result) in self
                .channels
                .iter()
                .map(|channel| (channel, channel.check_for_update(&api_key, last_checked)))
            {
                match update_result {
                    Ok(update) => {
                        if let Some(update) = update {
                            updates.push((channel.name.as_str(), update));
                        }
                    }
                    Err(error) => errors.push((channel.name.as_str(), error)),
                }
            }
        }

        (updates, errors)
    }
}

impl YouTubeChannel {
    pub fn check_for_update(
        &self,
        api_key: &str,
        last_checked: &Option<DateTime<Local>>,
    ) -> Result<Option<String>, String> {
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

        let data: Value = reqwest::get(&query)
            .map_err(|_err| format!("Couldn't access {}", query))?
            .json()
            .map_err(|_err| "Couldn't parse request data as JSON".to_owned())?;

        let items: &Vec<Value> = data
            .pointer("/items")
            .and_then(|obj| obj.as_array())
            .ok_or("YouTube API JSON data wasn't an object")?;

        let mut updates = items
            .into_iter()
            .filter_map(|item| {
                let pub_date_str = item
                    .pointer("/snippet/publishedAt")
                    .and_then(|date_obj| date_obj.as_str())?;
                let pub_date = DateTime::<FixedOffset>::parse_from_rfc3339(pub_date_str)
                    .map(|date| date.with_timezone(&Local))
                    .ok()?;
                let title = item
                    .pointer("/snippet/title")
                    .and_then(|title_obj| title_obj.as_str())
                    .map(|title| title)
                    .unwrap_or("<unnamed>")
                    .to_owned();
                let link = item
                    .pointer("/id/videoId")
                    .and_then(|id_obj| id_obj.as_str())
                    .map(|id| format!("https://www.youtube.com/watch?v={}", id))
                    .unwrap_or("<no link>".to_owned());

                Some((title, link, pub_date))
            })
            .collect::<Vec<(String, String, DateTime<Local>)>>();
        updates.sort_by_key(|(_title, _link, pub_date)| pub_date.clone());
        let num_updates = updates.len();

        Ok(updates
            .into_iter()
            .map(|(title, link, pub_date)| update_message(num_updates, &title, &link, &pub_date))
            .next())
    }
}
