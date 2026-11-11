//! Command-line interface. Commands have English names; the original Turkish names are kept
//! as visible aliases.

use clap::{Parser, Subcommand};

/// End-to-end encrypted clipboard sync between your devices.
#[derive(Parser)]
#[command(name = "panosync", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Command {
    /// Set up this device. Generates a new key if none is given.
    #[command(visible_alias = "kurulum")]
    Setup {
        /// Relay server address, e.g. wss://panosync.example.com
        #[arg(long, visible_alias = "sunucu", value_name = "URL")]
        server: String,
        /// Key generated on your first device (leave empty on the first device)
        #[arg(long, visible_alias = "anahtar", value_name = "KEY")]
        key: Option<String>,
    },
    /// Start syncing (the default when no command is given).
    #[command(visible_alias = "baslat")]
    Start,
    /// Print the key, to add another device.
    #[command(visible_alias = "anahtar")]
    Key,
    /// Print the location of the config file.
    #[command(visible_alias = "ayar")]
    Config,
    /// Stop sending clipboard changes for the given number of minutes (0 resumes).
    #[command(visible_alias = "duraklat")]
    Pause {
        /// Minutes to pause for. 0 resumes immediately.
        #[arg(value_name = "MINUTES")]
        minutes: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Option<Command> {
        Cli::try_parse_from(std::iter::once("panosync").chain(args.iter().copied()))
            .unwrap()
            .command
    }

    #[test]
    fn english_and_turkish_names_are_the_same_command() {
        let expected = Some(Command::Setup {
            server: "wss://p.example.com".into(),
            key: Some("abc".into()),
        });
        assert_eq!(
            parse(&["setup", "--server", "wss://p.example.com", "--key", "abc"]),
            expected
        );
        assert_eq!(
            parse(&[
                "kurulum",
                "--sunucu",
                "wss://p.example.com",
                "--anahtar",
                "abc"
            ]),
            expected
        );

        for (english, turkish, command) in [
            ("start", "baslat", Command::Start),
            ("key", "anahtar", Command::Key),
            ("config", "ayar", Command::Config),
        ] {
            assert_eq!(parse(&[english]), Some(command));
            assert_eq!(parse(&[english]), parse(&[turkish]));
        }
        assert_eq!(parse(&[]), None);
    }

    #[test]
    fn pause_takes_minutes() {
        assert_eq!(
            parse(&["pause", "15"]),
            Some(Command::Pause { minutes: 15 })
        );
        assert_eq!(
            parse(&["duraklat", "0"]),
            Some(Command::Pause { minutes: 0 })
        );
        assert!(Cli::try_parse_from(["panosync", "pause"]).is_err());
        assert!(Cli::try_parse_from(["panosync", "pause", "-3"]).is_err());
    }
}
