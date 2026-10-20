//! Native installer asset and macOS disk-image tools.
//!
//! `installers art [--review DIR]` replaces `scripts/installers/make-art.py`.
//! `installers dmg APP BACKGROUND ICON OUT` replaces the Python dmgbuild
//! runtime/settings, keeping HFS+, ULFO, `HiDPI` artwork and headless Finder
//! layout. `installers check-dmg-layout MOUNT` verifies that layout offline.
//! `installers tools DIR TARGET` fetches the pinned installer tools.
use anyhow::{Result, bail};
use std::path::Path;

mod art;
mod dmg;
mod layout;
mod tools;

/// A path as a Windows program reads it: backslashes, and without the
/// `\\?\` prefix `fs::canonicalize` gives on Windows, which installers such
/// as Inno Setup's refuse (`\\?\UNC\host\share` becomes `\\host\share`).
pub(crate) fn windows_path(path: &Path) -> String {
    let text = path.display().to_string().replace('/', "\\");
    if let Some(share) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{share}")
    } else if let Some(local) = text.strip_prefix(r"\\?\") {
        local.to_owned()
    } else {
        text
    }
}

const HELP: &str = "Usage: baylee-tools installers <COMMAND>

Commands:
  art [--review DIR]              Regenerate committed installer art and icons
                                 Optionally write an icon comparison sheet
  dmg APP BACKGROUND ICON OUT     Build an HFS+/ULFO disk image on macOS
                                 Includes the @2x background and Finder layout
  check-dmg-layout MOUNT          Verify a mounted image's Finder layout
  tools DIR TARGET                Fetch this system's pinned installer tools
                                 Prints NAME=value lines for GITHUB_ENV

Relative paths use the repository root.";

/// Runs an installer tool relative to the repository root.
///
/// # Errors
/// Returns an error for invalid arguments, failed validation, or failed I/O.
pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let words: Vec<_> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["--help" | "-h"] => {
            println!("{HELP}");
            Ok(())
        }
        ["art"] => art::generate(root, None),
        ["art", "--review", directory] => art::generate(root, Some(&root.join(directory))),
        ["dmg", app, background, icon, output] => dmg::build(
            &root.join(app),
            &root.join(background),
            &root.join(icon),
            &root.join(output),
        ),
        ["check-dmg-layout", mount] => dmg::check_layout(&root.join(mount)),
        ["tools", directory, target] => tools::fetch(&root.join(directory), target),
        _ => bail!(
            "usage: installers art [--review DIR]|dmg APP BACKGROUND ICON OUT|check-dmg-layout MOUNT|tools DIR TARGET"
        ),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn help_aliases_succeed_without_generating_or_overwriting_assets() {
        let temporary = tempfile::tempdir().unwrap();
        let missing_root = temporary.path().join("absent");
        for flag in ["--help", "-h"] {
            super::run(&missing_root, &[flag.into()]).unwrap();
        }
        assert!(!missing_root.exists());
        assert!(super::run(&missing_root, &["unknown-command".into()]).is_err());
    }

    #[test]
    fn a_windows_path_has_no_verbatim_prefix() {
        use super::windows_path;
        use std::path::Path;

        for (given, wanted) in [
            (r"\\?\D:\a\_temp\inno", r"D:\a\_temp\inno"),
            (r"\\?\UNC\host\share\inno", r"\\host\share\inno"),
            ("D:/a/_temp/inno", r"D:\a\_temp\inno"),
            (r"D:\plain", r"D:\plain"),
        ] {
            assert_eq!(windows_path(Path::new(given)), wanted, "{given}");
        }
    }
}
