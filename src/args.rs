use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, TimeZone};
use std::path::PathBuf;
use structopt::StructOpt;

/// A tool for keeping you updated on your news.
///
/// Just run it with no arguments to see what you've missed
/// and it will remember when you last ran it.
/// The currently allowed sources are RSS and YouTube. You can
/// manage your sources via the subcommands shown below.
#[derive(StructOpt)]
pub struct Args {
    /// The location of your config.json file. If not specified,
    /// one is managed in your system's config directory.
    #[structopt(short = "c", long = "config", parse(from_os_str))]
    pub config: Option<PathBuf>,

    /// If you want to check for updates from a specific date (and time) on
    /// instead of from the last time this was run, specify one here.
    /// Allowed formats are:
    ///
    /// ["today", "yesterday", "MM/DD/YYYY", "HH:MM (AM|PM) MM/DD/YYYY"]
    #[structopt(
        short = "t",
        long = "since-time",
        parse(try_from_str = "parse_arg_time")
    )]
    pub since_time: Option<DateTime<Local>>,

    /// The optional subcommands for editing your source list.
    #[structopt(subcommand)]
    pub command: Option<Command>,
}

#[derive(StructOpt)]
pub enum Command {
    /// Manage your RSS feeds.
    #[structopt(name = "rss")]
    Rss(RssCommand),

    /// Manage your YouTube channels.
    #[structopt(name = "youtube")]
    YouTube(YouTubeCommand),
}

#[derive(StructOpt)]
pub enum RssCommand {
    /// Add an RSS feed to sitch. You can provide all, none,
    /// or some of the arguments for the given type, sitch will
    /// open your preferred editor to fill in the rest of a JSON
    /// object if you missed any required fields.
    #[structopt(name = "add")]
    Add {
        /// Your name for the feed.
        #[structopt(short = "n", long = "name")]
        name: Option<String>,

        /// The URL of the feed location.
        #[structopt(short = "f", long = "feed")]
        feed: Option<String>,
    },

    /// List your RSS feeds.
    #[structopt(name = "list")]
    List,

    /// Edit your current RSS feeds in your favorite editor. Requires
    /// the EDITOR environment variable to be set.
    #[structopt(name = "edit")]
    Edit,
}

#[derive(StructOpt)]
pub enum YouTubeCommand {
    /// Add a YouTube channel to sitch. You can provide all, none,
    /// or some of the arguments for the given type, sitch will
    /// open your preferred editor to fill in the rest of a JSON
    /// object if you missed any required fields.
    #[structopt(name = "add")]
    Add {
        /// The name of the YouTube channel.
        #[structopt(short = "n", long = "name")]
        name: Option<String>,

        /// The channel ID as found on each channel's home page in the URL.
        #[structopt(short = "i", long = "id")]
        channel_id: Option<String>,
    },

    /// List your YouTube channels.
    #[structopt(name = "list")]
    List,

    /// Edit your current YouTube channels in your favorite editor. Requires
    /// the EDITOR environment variable to be set.
    #[structopt(name = "edit")]
    Edit,

    /// Manage the YouTube API key (required for sitch to access the YouTube API).
    /// If the key is set, sitch will check the channels for recent videos. If it
    /// is never set or it is cleared, then sitch will ignore the YouTube feature.
    /// To acquire an API key, follow this link:
    /// https://developers.google.com/youtube/v3/getting-started
    #[structopt(name = "apikey")]
    ApiKey(YouTubeApiCommand),
}

#[derive(StructOpt)]
pub enum YouTubeApiCommand {
    /// Set the API key.
    #[structopt(name = "set")]
    Set {
        /// The new API key to use for checking YouTube.
        #[structopt(short = "k", long = "key")]
        new_key: String,
    },

    /// Clear the existing key (if you want sitch to ignore YouTube channels).
    #[structopt(name = "clear")]
    Clear,

    /// Show your current key if it is set (prints nothing if no key is set).
    #[structopt(name = "show")]
    Show,
}

fn parse_arg_time(date_str: &str) -> Result<DateTime<Local>, String> {
    if date_str == "today" {
        Ok(Local::today().and_hms(0, 0, 0))
    } else if date_str == "yesterday" {
        Ok(Local::today().and_hms(0, 0, 0) - Duration::days(1))
    } else if let Ok(naive_date) = NaiveDate::parse_from_str(date_str, "%-m/%e/%Y") {
        Ok(Local
            .from_local_datetime(&naive_date.and_hms(0, 0, 0))
            .earliest()
            .expect("Couldn't find timezone"))
    } else if let Ok(naive_datetime) =
        NaiveDateTime::parse_from_str(date_str, "%-l:%M %p %-m/%e/%Y")
    {
        Ok(Local
            .from_local_datetime(&naive_datetime)
            .earliest()
            .expect("Couldn't find timezone"))
    } else {
        Err("Could not parse the provided time. \
             Make sure it is one of the allowed formats."
            .to_owned())
    }
}
