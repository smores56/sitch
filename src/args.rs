//! Argument parsing for command-line usage.

use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, TimeZone};
use std::path::PathBuf;
use structopt::StructOpt;

/// A tool for keeping you updated.
///
/// Just run it with no arguments to see what you've missed
/// and it will remember when you last ran it. The currently
/// allowed sources are RSS, YouTube, Gmail, Anime, and Manga.
/// You can manage your sources via the subcommands shown below.
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

    /// For linux systems, send the output as clickable notifications instead.
    #[structopt(long = "notify")]
    pub notify: bool,

    /// Run in quiet mode, or simplify the output.
    #[structopt(short = "q", long = "quiet")]
    pub quiet: bool,

    /// Only output the last time sitch checked for updates.
    /// The format is "HH:MM:SS MM/DD/YY" (24 hour)
    #[structopt(short = "L", long = "last-checked")]
    pub last_checked: bool,

    /// The optional subcommands for editing your source list.
    #[structopt(subcommand)]
    pub command: Option<Command>,
}

#[derive(StructOpt)]
pub enum Command {
    /// Manage your RSS feeds.
    #[structopt(name = "rss")]
    Rss(RssCommand),

    /// Manage your Bandcamp artists.
    #[structopt(name = "bandcamp")]
    Bandcamp(BandcampCommand),

    /// Manage your YouTube channels.
    #[structopt(name = "youtube")]
    YouTube(YouTubeCommand),

    /// Manage the manga you follow.
    #[structopt(name = "manga")]
    Manga(MangaCommand),

    /// Manage the anime you follow.
    #[structopt(name = "anime")]
    Anime(AnimeCommand),
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
pub enum BandcampCommand {
    /// Add an Bandcamp artist to sitch. You can provide all, none,
    /// or some of the arguments for the given type, sitch will
    /// open your preferred editor to fill in the rest of a JSON
    /// object if you missed any required fields.
    #[structopt(name = "add")]
    Add {
        /// Your name for the artist.
        #[structopt(short = "n", long = "name")]
        name: Option<String>,

        /// The URL of the bandcamp page.
        #[structopt(short = "u", long = "url")]
        url: Option<String>,
    },

    /// List your Bandcamp artists.
    #[structopt(name = "list")]
    List,

    /// Edit your current Bandcamp artists in your favorite editor.
    /// Requires the EDITOR environment variable to be set.
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

    /// Interactively search for YouTube channels and add the channel
    /// you want correctly to sitch without needing a web browser.
    #[structopt(name = "search")]
    Search,

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

#[derive(StructOpt)]
pub enum GmailCommand {
    /// Add a Gmail search filter to sitch. Use this link to learn
    /// how to build filters in Gmail:
    /// https://support.google.com/mail/answer/7190?hl=en
    #[structopt(name = "add")]
    Add {
        /// The filter to search with.
        #[structopt(short = "f", long = "filter")]
        filter: String,
    },

    /// List your Gmail filters.
    #[structopt(name = "list")]
    List,

    /// Edit your current Gmail filters in your favorite editor. Requires
    /// the EDITOR environment variable to be set.
    #[structopt(name = "edit")]
    Edit,

    /// Manage the Gmail API Oauth (required for sitch to access the
    /// Gmail API). If the client ID is set, sitch will check each of the
    /// filters. If it is never set or it is cleared, then sitch will
    /// ignore the Gmail feature. To acquire a client ID, follow this link:
    /// https://console.developers.google.com/flows/enableapi?apiid=gmail
    #[structopt(name = "apikey")]
    ApiKey(GmailOauthCommand),
}

#[derive(StructOpt)]
pub enum GmailOauthCommand {
    /// Set the client ID. You can either specify the location of a JSON
    /// file or pipe JSON data in through stdin.
    #[structopt(name = "set")]
    Set {
        /// The location of the client ID file you downloaded from Google.
        #[structopt(short = "l", long = "location", parse(from_os_str))]
        location: Option<PathBuf>,
    },

    /// Clear the existing Oauth (if you want sitch to ignore your Gmail).
    #[structopt(name = "clear")]
    Clear,

    /// Show your current Oauth if it is set (prints nothing if no key is set).
    #[structopt(name = "show")]
    Show,
}

#[derive(StructOpt)]
pub enum AnimeCommand {
    /// Add an anime to sitch. You can provide all, none,
    /// or some of the arguments for the given type, sitch will
    /// open your preferred editor to fill in the rest of a JSON
    /// object if you missed any required fields.
    ///
    /// It is recommended to use the search subcommand instead, as
    /// it will find the appropriate id for you, rather than making
    /// you find the correct one.
    #[structopt(name = "add")]
    Add {
        /// The name of the anime.
        #[structopt(short = "n", long = "name")]
        name: Option<String>,

        /// The id of the anime as found on "myanimelist.net".
        #[structopt(short = "i", long = "id")]
        id: Option<String>,
    },

    /// List the anime you follow.
    #[structopt(name = "list")]
    List,

    /// Edit your currently followed anime in your favorite editor. Requires
    /// the EDITOR environment variable to be set.
    #[structopt(name = "edit")]
    Edit,

    /// Interactively search for anime on "myanimelist.net" and add the
    /// anime you want correctly to sitch without needing a web browser.
    #[structopt(name = "search")]
    Search,
}

#[derive(StructOpt)]
pub enum MangaCommand {
    /// Add a manga to sitch. You can provide all, none,
    /// or some of the arguments for the given type, sitch will
    /// open your preferred editor to fill in the rest of a JSON
    /// object if you missed any required fields.
    ///
    /// It is recommended to use the search subcommand instead, as
    /// it will find the appropriate id for you, rather than making
    /// you find the correct one.
    #[structopt(name = "add")]
    Add {
        /// The name of the manga.
        #[structopt(short = "n", long = "name")]
        name: Option<String>,

        /// The id of the manga as found on "mangaeden.com".
        #[structopt(short = "i", long = "id")]
        id: Option<String>,
    },

    /// List the manga you follow.
    #[structopt(name = "list")]
    List,

    /// Edit your currently followed manga in your favorite editor. Requires
    /// the EDITOR environment variable to be set.
    #[structopt(name = "edit")]
    Edit,

    /// Interactively search for manga on "mangaeden.com" and add the
    /// manga you read correctly to sitch without needing a web browser.
    #[structopt(name = "search")]
    Search,
}

/// Attempts to parse the `since_time` command-line argument.
///
/// If the date/time can be interpretted by one of the below
/// formats, the datetime is used as a starting point to search
/// for updates since.
///
/// The possible formats are:
/// - The literal strings "today" or "yesterday"
/// - A date in the format "MM/DD/YYYY"
/// - A date and time in the format "HH:MM (AM|PM) MM/DD/YYYY"
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
