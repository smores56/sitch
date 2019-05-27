//! Sitch keeps you updated on what you follow. It currently
//! supports the following sources:
//! - YouTube channels
//! - RSS feeds
//! - Anime (myanimelist.net via Jikan)
//! - Manga (mangaeden.net API)
//! - Bandcamp artists
//!
//! Read more on the [sitch repository](https://www.github.com/smores56/sitch).

extern crate atty;
extern crate chrono;
extern crate colored;
extern crate dirs;
extern crate notify_rust;
extern crate rayon;
extern crate reqwest;
extern crate rss;
extern crate select;
extern crate serde;
extern crate serde_json;
extern crate structopt;
extern crate webbrowser;

pub mod args;
pub mod sources;
pub mod util;

use chrono::{DateTime, Local};
use colored::Colorize;
use serde::Deserialize;
use serde_json::json;
use std::process;
use structopt::StructOpt;
use util::edit_as_json;

use args::{
    AnimeCommand, Args, BandcampCommand, Command, MangaCommand, RssCommand, YouTubeApiCommand,
    YouTubeCommand,
};
use sources::anime::Anime;
use sources::bandcamp::BandcampArtist;
use sources::manga::Manga;
use sources::rss::RssSource;
use sources::youtube::YouTubeChannel;
use sources::Sources;

fn run() -> Result<(), String> {
    // parse arguments
    let args = Args::from_args();
    // load source configuration file
    let mut sources = Sources::load(args.config.clone())?;
    // if just checking the last time it was run,
    if args.last_checked {
        if let Some(last_checked) = sources.last_checked {
            // either print the date and exit gracefully,
            println!("{}", last_checked.format("%T %D"));
            std::process::exit(0);
        } else {
            // or print an error and exit accordingly.
            eprintln!("sitch has not successfully run yet.");
            std::process::exit(1);
        }
    }
    // overwrite the last time run if one was specified
    if let Some(since_time) = args.since_time {
        sources.last_checked = Some(since_time);
    }

    if let Some(command) = args.command {
        match command {
            Command::Rss(rss_command) => match rss_command {
                RssCommand::Add { name, feed } => {
                    // if both name and feed url are provided,
                    if name.is_some() && feed.is_some() {
                        // add the new rss source to sitch
                        sources.rss.0.push((
                            RssSource {
                                name: name.unwrap(),
                                feed: feed.unwrap(),
                            },
                            None,
                        ));
                    } else {
                        // otherwise, let the user edit a JSON object in their
                        // preferred editor and attempt to save the edited JSON as
                        // an new rss source
                        edit_as_json(&json!({ "name": name, "feed": feed }), |edited| {
                            let source = RssSource::deserialize(edited).map_err(|err| {
                                format!("The edited object could not be parsed: {}.", err)
                            })?;
                            sources.rss.0.push((source, None));
                            Ok(())
                        })?;
                    }
                    println!("Added a new RSS feed.");
                }
                RssCommand::List => {
                    for (source, _last_checked) in &sources.rss.0 {
                        // only print color if the output isn't piped
                        if atty::is(atty::Stream::Stdout) {
                            println!("{}: {}", source.name.green(), source.feed.bright_blue());
                        } else {
                            println!("{}: {}", source.name, source.feed);
                        }
                    }
                }
                RssCommand::Edit => {
                    // attempt to edit all of the user's rss sources in their
                    // preferred editor, and save if the edit was successful
                    edit_as_json(&sources.rss.clone(), |edited| {
                        let rss = Vec::<(RssSource, Option<DateTime<Local>>)>::deserialize(edited)
                            .map_err(|err| {
                                format!("The edited RSS sources could not be parsed: {}.", err)
                            })?;
                        sources.rss.0 = rss;
                        Ok(())
                    })?;
                }
            },
            Command::Bandcamp(bandcamp_command) => match bandcamp_command {
                BandcampCommand::Add { name, url } => {
                    // if both name and artist url are provided,
                    if name.is_some() && url.is_some() {
                        // add the new bandcamp artist to sitch
                        sources.bandcamp.0.push((
                            BandcampArtist {
                                name: name.unwrap(),
                                url: url.unwrap(),
                            },
                            None,
                        ));
                    } else {
                        // otherwise, let the user edit a JSON object in their
                        // preferred editor and attempt to save the edited JSON as
                        // an new bandcamp artist
                        edit_as_json(&json!({ "name": name, "url": url }), |edited| {
                            let source = BandcampArtist::deserialize(edited).map_err(|err| {
                                format!("The edited object could not be parsed: {}.", err)
                            })?;
                            sources.bandcamp.0.push((source, None));
                            Ok(())
                        })?;
                    }
                    println!("Added a new Bandcamp artist.");
                }
                BandcampCommand::List => {
                    for (source, _last_checked) in &sources.bandcamp.0 {
                        // only print color if the output isn't piped
                        if atty::is(atty::Stream::Stdout) {
                            println!("{}: {}", source.name.green(), source.url.bright_blue());
                        } else {
                            println!("{}: {}", source.name, source.url);
                        }
                    }
                }
                BandcampCommand::Edit => {
                    // attempt to edit all of the user's bandcamp artists in their
                    // preferred editor, and save if the edit was successful
                    edit_as_json(&sources.bandcamp.clone(), |edited| {
                        let artists =
                            Vec::<(BandcampArtist, Option<DateTime<Local>>)>::deserialize(edited)
                                .map_err(|err| {
                                format!("The edited bandcamp artists could not be parsed: {}.", err)
                            })?;
                        sources.bandcamp.0 = artists;
                        Ok(())
                    })?;
                }
            },
            Command::YouTube(youtube_command) => match youtube_command {
                // if both name and channel id are provided,
                YouTubeCommand::Add { name, channel_id } => {
                    // then add the new YouTube channel to sitch
                    if name.is_some() && channel_id.is_some() {
                        sources.youtube.channels.push((
                            YouTubeChannel {
                                name: name.unwrap(),
                                channel_id: channel_id.unwrap(),
                            },
                            None,
                        ));
                    } else {
                        // otherwise, let the user edit a JSON object in their
                        // preferred editor and attempt to save the edited JSON as
                        // an new YouTube channel
                        edit_as_json(
                            &json!({ "name": name, "channel_id": channel_id }),
                            |edited| {
                                let channel =
                                    YouTubeChannel::deserialize(edited).map_err(|err| {
                                        format!("The edited object could not be parsed: {}.", err)
                                    })?;
                                sources.youtube.channels.push((channel, None));
                                Ok(())
                            },
                        )?;
                    }
                    println!("Added a new YouTube channel.");
                }
                YouTubeCommand::List => {
                    for (channel, _last_checked) in &sources.youtube.channels {
                        // only print color if the output isn't piped
                        if atty::is(atty::Stream::Stdout) {
                            println!("{}: {}", channel.name.green(), channel.channel_id);
                        } else {
                            println!("{}: {}", channel.name, channel.channel_id);
                        }
                    }
                }
                YouTubeCommand::Edit => {
                    // attempt to edit all of the user's YouTube channels in their
                    // preferred editor, and save if the edit was successful
                    edit_as_json(&sources.youtube.channels.clone(), |edited| {
                        let channels =
                            Vec::<(YouTubeChannel, Option<DateTime<Local>>)>::deserialize(edited)
                                .map_err(|err| {
                                format!("The edited channels could not be parsed: {}.", err)
                            })?;
                        sources.youtube.channels = channels;
                        Ok(())
                    })?;
                }
                YouTubeCommand::Search => match sources.youtube.interactive_search() {
                    // search for channels, and if one is found and selected,
                    // add it to their config file
                    Ok(new_channel) => {
                        sources.youtube.channels.push((new_channel, None));
                        println!("Added a new channel.");
                    }
                    // otherwise, print the returned error message
                    Err(err) => eprintln!("{}", err),
                },
                YouTubeCommand::ApiKey(api_command) => match api_command {
                    // set or update the required API key for YouTube channel updates
                    YouTubeApiCommand::Set { new_key } => sources.youtube.api_key = Some(new_key),
                    // clear the key
                    YouTubeApiCommand::Clear => sources.youtube.api_key = None,
                    // if a key exists, print it
                    YouTubeApiCommand::Show => {
                        if let Some(key) = &sources.youtube.api_key {
                            println!("{}", key);
                        }
                    }
                },
            },
            Command::Anime(anime_command) => match anime_command {
                // if both a name and anime id were provided,
                AnimeCommand::Add { name, id } => {
                    if name.is_some() && id.is_some() {
                        // add the new anime to sitch
                        sources.anime.0.push((
                            Anime {
                                name: name.unwrap(),
                                id: id.unwrap(),
                            },
                            None,
                        ));
                    } else {
                        // otherwise, let the user edit a JSON object in their
                        // preferred editor and attempt to save the edited JSON as
                        // an new anime
                        edit_as_json(&json!({ "name": name, "id": id }), |edited| {
                            let anime = Anime::deserialize(edited).map_err(|err| {
                                format!("The edited object could not be parsed: {}.", err)
                            })?;
                            sources.anime.0.push((anime, None));
                            Ok(())
                        })?;
                        println!("Added a new anime.");
                    }
                }
                AnimeCommand::List => {
                    for (anime, _last_checked) in &sources.anime.0 {
                        println!("{}", anime.name);
                    }
                }
                AnimeCommand::Edit => {
                    // attempt to edit all of the user's anime in their
                    // preferred editor, and save if the edit was successful
                    edit_as_json(&sources.anime.clone(), |edited| {
                        let anime = Vec::<(Anime, Option<DateTime<Local>>)>::deserialize(edited)
                            .map_err(|err| {
                                format!("The edited anime could not be parsed: {}.", err)
                            })?;
                        sources.anime.0 = anime;
                        Ok(())
                    })?;
                }
                AnimeCommand::Search => match Anime::interactive_search() {
                    // search for anime, and if one is found and selected,
                    // add it to their config file
                    Ok(new_anime) => {
                        sources.anime.0.push((new_anime, None));
                        println!("Added a new anime.");
                    }
                    // otherwise, print the returned error message
                    Err(err) => eprintln!("{}", err),
                },
            },
            Command::Manga(manga_command) => match manga_command {
                // if both a name and manga id were provided,
                MangaCommand::Add { name, id } => {
                    if name.is_some() && id.is_some() {
                        // add the new manga to sitch
                        sources.manga.0.push((
                            Manga {
                                name: name.unwrap(),
                                id: id.unwrap(),
                            },
                            None,
                        ));
                    } else {
                        // otherwise, let the user edit a JSON object in their
                        // preferred editor and attempt to save the edited JSON as
                        // an new manga
                        edit_as_json(&json!({ "name": name, "id": id }), |edited| {
                            let manga = Manga::deserialize(edited).map_err(|err| {
                                format!("The edited object could not be parsed: {}.", err)
                            })?;
                            sources.manga.0.push((manga, None));
                            Ok(())
                        })?;
                        println!("Added a new manga.");
                    }
                }
                MangaCommand::List => {
                    for (manga, _last_checked) in &sources.manga.0 {
                        println!("{}", manga.name);
                    }
                }
                MangaCommand::Edit => {
                    // attempt to edit all of the user's manga in their
                    // preferred editor, and save if the edit was successful
                    edit_as_json(&sources.manga.clone(), |edited| {
                        let manga = Vec::<(Manga, Option<DateTime<Local>>)>::deserialize(edited)
                            .map_err(|err| {
                                format!("The edited manga could not be parsed: {}.", err)
                            })?;
                        sources.manga.0 = manga;
                        Ok(())
                    })?;
                }
                MangaCommand::Search => match Manga::interactive_search() {
                    // search for anime, and if one is found and selected,
                    // add it to their config file
                    Ok(new_manga) => {
                        sources.manga.0.push((new_manga, None));
                        println!("Added a new manga.");
                    }
                    // otherwise, print the returned error message
                    Err(err) => eprintln!("{}", err),
                },
            },
        }
    } else {
        // if no subcommand was provided, check for updates
        sources.check_for_updates(args.quiet, args.notify);
    }

    // if an error hasn't occured yet, save potential changes
    sources.save(args.config)?;

    Ok(())
}

fn main() {
    // handle errors above gracefully
    if let Err(error) = run() {
        eprintln!("{}", error);
        process::exit(1);
    }
}
