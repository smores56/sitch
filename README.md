# Sitch #

Sitch keeps you updated on what you follow. It currently
supports the following sources:
- YouTube channels
- RSS feeds

Sitch is written in [Rust](https://www.rust-lang.org/) (stable).


## Installation ##

To install sitch, you neeed to have [rust](https://rustup.rs/)
installed. Clone the repo and then use `cargo` to install from
the cloned folder:

```bash
cargo install
```


## Usage ##

The usual way to run sitch is bare:

```bash
sitch
```

Sitch will remember when you last ran it and check for updates
since then, and let you know in a pretty format.

<!-- TODO: Give an example of output. -->


## Configuration ##

To manage your sources, you can run the subcommands (e.g. `rss`
or `youtube`) and they will explain how to manage them. You can
add, list, or bulk edit them. Some of them require specific
attention to get going.

In order to check YouTube channels for updates, sitch uses the
YouTube API v3. You'll need to follow
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
[MIT](https://choosealicense.com/licenses/mit/) license.
