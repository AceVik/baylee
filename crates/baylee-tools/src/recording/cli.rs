//! Command-line shape shared by the standalone helper and its tests.
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "recording",
    about = "Local render recording, animation previews and frame diagnostics"
)]
pub(super) struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub(super) enum Command {
    /// Record rendered frames through loopback dev-control, then encode.
    Record {
        #[arg(long, default_value_t = 28770, value_parser = clap::value_parser!(u16).range(1..))]
        port: u16,
        #[arg(long, default_value_t = 3000, value_parser = clap::value_parser!(u32).range(1..=30000))]
        duration_ms: u32,
        #[arg(long, default_value_t = 15, value_parser = clap::value_parser!(u32).range(1..=60))]
        fps: u32,
        #[command(flatten)]
        preview: Preview,
    },
    /// Encode a completed local PNG/JSON recording without contacting a client.
    Encode {
        directory: PathBuf,
        #[arg(long)]
        allow_incomplete: bool,
        #[command(flatten)]
        preview: Preview,
    },
    /// Read current recorder status.
    Status {
        #[arg(long, default_value_t = 28770, value_parser = clap::value_parser!(u16).range(1..))]
        port: u16,
    },
    /// Stop sampling and drain, or discard pending samples with --cancel.
    Stop {
        #[arg(long, default_value_t = 28770, value_parser = clap::value_parser!(u16).range(1..))]
        port: u16,
        #[arg(long)]
        cancel: bool,
    },
}

#[derive(Args, Clone, Debug)]
pub(super) struct Preview {
    #[arg(long, value_enum, default_value = "webp")]
    pub format: Format,
    #[arg(long, default_value_t = 640, value_parser = clap::value_parser!(u32).range(16..=1920))]
    pub width: u32,
    #[arg(long, value_enum, default_value = "real")]
    pub cadence: Cadence,
    #[arg(long, default_value_t = 12, value_parser = clap::value_parser!(u32).range(1..=48))]
    pub sheet_frames: u32,
    /// Write heuristic full-resolution RGB differences.csv (not bug proof).
    #[arg(long)]
    pub diff: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(super) enum Format {
    Gif,
    Webp,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(super) enum Cadence {
    Real,
    Virtual,
    Nominal,
}

impl Cadence {
    pub fn name(self) -> &'static str {
        match self {
            Self::Real => "real",
            Self::Virtual => "virtual",
            Self::Nominal => "nominal",
        }
    }
}
