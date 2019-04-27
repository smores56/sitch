extern crate chrono;
extern crate colored;
extern crate dirs;
extern crate reqwest;
extern crate rss;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;
extern crate structopt;

pub mod args;
pub mod formats;
pub mod rss_source;
pub mod util;
pub mod youtube;

use args::{Args, Command, RssCommand, YouTubeApiCommand, YouTubeCommand};
use colored::Colorize;
use formats::Formats;
use rss_source::RssSource;
use serde::Deserialize;
use serde_json::json;
use std::process;
use structopt::StructOpt;
use util::edit_as_json;
use youtube::YouTubeChannel;

fn run() -> Result<(), String> {
    let args = Args::from_args();
    let mut formats = Formats::load(args.config.clone())?;
    if let Some(since_time) = args.since_time {
        formats.last_checked = Some(since_time);
    }

    if let Some(command) = args.command {
        match command {
            Command::Rss(rss_command) => match rss_command {
                RssCommand::Add { name, feed } => {
                    if name.is_some() && feed.is_some() {
                        formats.rss.push(RssSource {
                            name: name.unwrap(),
                            feed: feed.unwrap(),
                        });
                    } else {
                        edit_as_json(&json!({ "name": name, "feed": feed }), |edited| {
                            let source = RssSource::deserialize(edited).map_err(|err| {
                                format!("The edited object could not be parsed: {}.", err)
                            })?;
                            formats.rss.push(source);
                            Ok(())
                        })?;
                    }
                    println!("Added a new RSS feed.");
                }
                RssCommand::List => {
                    for source in &formats.rss {
                        println!("{}: {}", source.name.green(), source.feed);
                    }
                }
                RssCommand::Edit => {
                    edit_as_json(&formats.rss.clone(), |edited| {
                        let rss = Vec::<RssSource>::deserialize(edited).map_err(|err| {
                            format!("The edited RSS sources could not be parsed: {}.", err)
                        })?;
                        formats.rss = rss;
                        Ok(())
                    })?;
                }
            },
            Command::YouTube(youtube_command) => match youtube_command {
                YouTubeCommand::Add { name, channel_id } => {
                    if name.is_some() && channel_id.is_some() {
                        formats.youtube.channels.push(YouTubeChannel {
                            name: name.unwrap(),
                            channel_id: channel_id.unwrap(),
                        });
                    } else {
                        edit_as_json(
                            &json!({ "name": name, "channel_id": channel_id }),
                            |edited| {
                                let channel =
                                    YouTubeChannel::deserialize(edited).map_err(|err| {
                                        format!("The edited object could not be parsed: {}.", err)
                                    })?;
                                formats.youtube.channels.push(channel);
                                Ok(())
                            },
                        )?;
                    }
                    println!("Added a new YouTube channel.");
                }
                YouTubeCommand::List => {
                    for channel in &formats.youtube.channels {
                        println!("{}: {}", channel.name.green(), channel.channel_id);
                    }
                }
                YouTubeCommand::Edit => {
                    edit_as_json(&formats.youtube.channels.clone(), |edited| {
                        let channels =
                            Vec::<YouTubeChannel>::deserialize(edited).map_err(|err| {
                                format!("The edited channels could not be parsed: {}.", err)
                            })?;
                        formats.youtube.channels = channels;
                        Ok(())
                    })?;
                }
                YouTubeCommand::ApiKey(api_command) => match api_command {
                    YouTubeApiCommand::Set { new_key } => formats.youtube.api_key = Some(new_key),
                    YouTubeApiCommand::Clear => formats.youtube.api_key = None,
                    YouTubeApiCommand::Show => {
                        if let Some(key) = &formats.youtube.api_key {
                            println!("{}", key);
                        }
                    }
                },
            },
        }
    } else {
        formats.check_for_updates();
    }

    formats.save(args.config)?;

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{}", error);
        process::exit(1);
    }
}
