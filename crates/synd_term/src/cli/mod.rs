use std::path::PathBuf;

use clap::{Parser, Subcommand};
use ratatui::style::palette::tailwind;
use url::Url;

use crate::config;

mod clear;

#[derive(Copy, Clone, PartialEq, Eq, Debug, clap::ValueEnum)]
pub enum Palette {
    Slate,
    Gray,
    Zinc,
    Neutral,
    Stone,
    Red,
    Orange,
    Amber,
    Yellow,
    Lime,
    Green,
    Emerald,
    Teal,
    Cyan,
    Sky,
    Blue,
    Indigo,
    Violet,
    Purple,
    Fuchsia,
    Pink,
}

impl From<Palette> for tailwind::Palette {
    fn from(t: Palette) -> Self {
        #[allow(clippy::wildcard_imports)]
        match t {
            Palette::Slate => tailwind::SLATE,
            Palette::Gray => tailwind::GRAY,
            Palette::Zinc => tailwind::ZINC,
            Palette::Neutral => tailwind::NEUTRAL,
            Palette::Stone => tailwind::STONE,
            Palette::Red => tailwind::RED,
            Palette::Orange => tailwind::ORANGE,
            Palette::Amber => tailwind::AMBER,
            Palette::Yellow => tailwind::YELLOW,
            Palette::Lime => tailwind::LIME,
            Palette::Green => tailwind::GREEN,
            Palette::Emerald => tailwind::EMERALD,
            Palette::Teal => tailwind::TEAL,
            Palette::Cyan => tailwind::CYAN,
            Palette::Sky => tailwind::SKY,
            Palette::Blue => tailwind::BLUE,
            Palette::Indigo => tailwind::INDIGO,
            Palette::Violet => tailwind::VIOLET,
            Palette::Purple => tailwind::PURPLE,
            Palette::Fuchsia => tailwind::FUCHSIA,
            Palette::Pink => tailwind::PINK,
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, propagate_version = true)]
pub struct Args {
    /// synd_api endpoint
    #[arg(long, default_value = config::api::ENDPOINT, env = config::env::ENDPOINT)]
    pub endpoint: Url,
    /// Log file path
    #[arg(long, default_value = config::log_path().into_os_string(), env = config::env::LOG_PATH)]
    pub log: PathBuf,
    /// Color palette
    #[arg(value_enum, long = "theme", default_value_t = Palette::Slate, env = config::env::THEME)]
    pub palette: Palette,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Clear(clear::ClearCommand),
}

pub fn parse() -> Args {
    Args::parse()
}
