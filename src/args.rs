use clap::{ArgAction, Parser};
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Option<Subcommands>,

    /// The path to the notes directory
    #[clap(short, long, default_value = "notes")]
    pub notes_dir: PathBuf,

    /// The path to the config file
    #[clap(short, long)]
    pub config_file: Option<PathBuf>,

    /// The text editor to use. The default is the system's default text editor (EDITOR
    /// environment variable), and if that doesn't work, use nano.
    #[clap(short, long)]
    pub editor: Option<String>,

    #[clap(short, long, action = ArgAction::Count, default_value_t = 1)]
    pub verbose_level: u8,
}

#[derive(Parser, Debug)]
pub enum Subcommands {
    /// Scans for unencrypted files in the notes directory and encrypts them.
    EncryptUnencrypted,
}
