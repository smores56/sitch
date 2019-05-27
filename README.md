# Sitch #

Sitch keeps you updated on what you follow. It currently
supports the following sources:
- YouTube channels
- RSS feeds
- Anime (myanimelist.net via Jikan)
- Manga (mangaeden.net API)
- Bandcamp artists

Sitch is written in [Rust](https://www.rust-lang.org/) (stable).


## Installation ##

To install sitch, you need to have [rust](https://rustup.rs/)
installed. Install the latest release by running the following
with the rust package manager, `cargo`:

```bash
cargo install sitch
```


## Usage ##

The usual way to run sitch is bare:

```bash
your@machine:~$ sitch
The following sources have updated since April 02, 2019 at 12:00 AM:
YouTube Channel - Northernlion: There have been 25 updates, the earliest was "Northernlion Plays - Katana Zero - Episode 6 [Rewind]" released on May 22, 2019 at 3:00 PM, found here: https://www.youtube.com/watch?v=0yXR8mUphuI [2 seconds]
YouTube Channel - Joseph Anderson: There have been 2 updates, the earliest was "Hollow Knight DLC - Swansong for Silksong" released on April 30, 2019 at 2:34 PM, found here: https://www.youtube.com/watch?v=Ece-wZ6VjFw [2 seconds]
YouTube Channel - Jacob Collier: There have been 3 updates, the earliest was "Jacob Collier - DJESSE World Tour: Recap" released on April 7, 2019 at 3:10 PM, found here: https://www.youtube.com/watch?v=m56mYgDkvSw [2 seconds]
Bandcamp - Disasterpeace: There has been 1 update, it was "Under the Silver Lake by Disasterpeace" released on April 19, 2019 at 12:00 AM, found here: https://music.disasterpeace.com//album/under-the-silver-lake [43 seconds]
```

Sitch will remember when you last ran it and check for updates
since then, and let you know in a pretty format.

You can also run it with notifications (tested only on Linux):

```bash
sitch --notify
```

They are displayed in the following format:

```
+--------------------------------------+
| Sitch - <Source Name>                |
|                                      |
| First Update Title [Open in Browser] |
+--------------------------------------+
```


## Configuration ##

To manage your sources, you can run the subcommands (e.g. `rss`
or `youtube`) and they will explain how to manage them. You can
add, list, or bulk edit them. You can also search for anime,
manga, and YouTube channels. Try the following:

```bash
your@machine:~$ sitch youtube search
Search for an channel by name: Shnabubula
Found 5 results:
1: "Shnabubula" (id = UC9XtgFNeoDbjISzoJT0Qi9w)
2: "Shnabubula Archives" (id = UCAnR_x1Zw8QeOMKZNpPLmfA)
3: "Shnabubula - Topic" (id = UCafsTb5OoyWFAhDYvHvwrww)
4: "Red Tailed Fox & Shnabubula - Topic" (id = UClcNGxtDFYQ6koL5Diw-baw)
5: "Simply Retro DX" (id = UCW_quA3bcrfSl446z78Z0SQ)
Pick a result to add [1 to 5]: 1
Added a new channel.
```

Most of the sources are batteries included, but YouTube requires an
API key for checking for updates and for using the search functionality.
Sitch uses the YouTube API v3. You'll need to follow
[this link](https://developers.google.com/youtube/v3/getting-started)
to get started. You can use the following to set your API key
once you acquire one:

```bash
sitch youtube apikey set -k <YOUR KEY HERE>
```

If you don't set one, sitch will just ignore your YouTube
channels until you set one.


## License ##

Sitch is based on the
[MIT license](https://choosealicense.com/licenses/mit/).
