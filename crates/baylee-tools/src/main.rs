//! Native development tools, independent of the game engine and renderer.
mod art;
mod cardbatch;
mod forge;
mod hooks;
mod installers;
mod llm;
mod recording;
mod release;
mod shell;
mod site;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Baylee's native development tools", version)]
struct Cli {
    /// Repository checkout to operate on (the launcher supplies its own root).
    #[arg(long)]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Capture and inspect dev-control recordings (GIF, WebP, contact sheets).
    #[command(disable_help_flag = true)]
    Recording {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Drive and inspect an isolated development client.
    #[command(disable_help_flag = true)]
    Shell {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Verify CI provenance, release assets and Windows icons.
    #[command(disable_help_flag = true)]
    Release {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Build native installer artwork and disk images.
    #[command(disable_help_flag = true)]
    Installers {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Generate and check website data, stamps and private Impressum blobs.
    #[command(disable_help_flag = true)]
    Site {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Read external adventure saves and resolve deck printing choices.
    #[command(disable_help_flag = true)]
    Forge {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run bounded card implementation and review lanes.
    #[command(disable_help_flag = true)]
    Llm {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Build original scene assets and the pinned orchestra sample bank.
    #[command(disable_help_flag = true)]
    Art {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Claude's card-test debt bookkeeping hooks.
    #[command(disable_help_flag = true)]
    Hooks {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Read a structured card-batch verdict.
    #[command(disable_help_flag = true)]
    Cardbatch {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn checkout(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|path| {
            path.join("Cargo.toml").is_file()
                && path.join("crates/baylee-core/Cargo.toml").is_file()
        })
        .map(Path::to_owned)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let root = cli
        .root
        .or_else(|| std::env::current_dir().ok().and_then(|cwd| checkout(&cwd)))
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(Path::parent)
                .expect("baylee-tools lives in crates/baylee-tools")
                .to_owned()
        });
    let root = root.as_path();
    let result = match cli.command {
        Command::Recording { args } => recording::run(root, &args),
        Command::Shell { args } => shell::run(root, &args),
        Command::Release { args } => release::run(root, &args),
        Command::Installers { args } => installers::run(root, &args),
        Command::Site { args } => site::run(root, &args),
        Command::Forge { args } => forge::run(root, &args),
        Command::Llm { args } => llm::run(root, &args),
        Command::Art { args } => art::run(root, &args),
        Command::Hooks { args } => hooks::run(root, &args),
        Command::Cardbatch { args } => cardbatch::run(root, &args),
    };
    if let Err(error) = &result
        && let Some(cli_error) = error.downcast_ref::<clap::Error>()
    {
        cli_error.exit();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_checkout_does_not_consume_child_flags() {
        let cli = Cli::try_parse_from([
            "baylee-tools",
            "--root",
            "/selected/checkout",
            "recording",
            "status",
            "--port",
            "29234",
        ])
        .unwrap();
        assert_eq!(cli.root, Some(PathBuf::from("/selected/checkout")));
        let Command::Recording { args } = cli.command else {
            panic!("wrong command");
        };
        assert_eq!(args, ["status", "--port", "29234"]);
    }

    #[test]
    fn default_checkout_is_the_callers_nearest_baylee_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("other checkout");
        std::fs::create_dir_all(root.join("crates/baylee-core")).unwrap();
        std::fs::write(root.join("Cargo.toml"), "[workspace]").unwrap();
        std::fs::write(root.join("crates/baylee-core/Cargo.toml"), "[package]").unwrap();
        let cwd = root.join("nested/work");
        std::fs::create_dir_all(&cwd).unwrap();
        assert_eq!(checkout(&cwd), Some(root));
        assert_eq!(checkout(dir.path()), None);
    }
}
