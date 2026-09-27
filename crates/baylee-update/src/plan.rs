//! Which renames replace an installation, per system.
//!
//! A pure function over names, so what Windows, macOS and Linux each get is
//! tested on every machine the tests run on: nothing here touches a file,
//! and a path is a list of names rather than a `PathBuf`, which on this
//! machine could not even spell the other system's.
//!
//! Everything is relative to the installation's **base**, the directory
//! the player unpacked the archive into: the folder holding
//! `baylee-client(.exe)` and `assets/` on Windows and Linux, and the folder
//! holding `Baylee.app` on macOS. The updater stages in `<base>/`[`STAGE`]:
//! the archive, and the tree it unpacks to under [`NEW`], its versioned
//! top folder already stripped.
//!
//! Each entry the new tree brings is replaced in two renames: the old one
//! is set aside in the same directory, then the new one moves in. Same
//! directory, because renaming a directory into a different parent needs
//! write access to the directory itself (to rewrite its `..`), which a
//! bundle an administrator copied into `/Applications` does not give.
//!
//! - **Windows** cannot overwrite or delete a running `.exe` but can rename
//!   it, so it becomes `baylee-client.old.exe` and is deleted at the next
//!   start. Everything else is set aside as `<name>.old`.
//! - **macOS** replaces the bundle as a whole. The files the archive
//!   carries beside it (`LICENSE`, `NOTICE`, `README.txt`, the `.dSYM`) are
//!   replaced only where they already sit beside the bundle: a bare
//!   `Baylee.app` in `/Applications` gets no litter dropped next to it.
//! - **Linux** replaces the binary and everything beside it, like Windows,
//!   without Windows' restriction on the running file.
//!
//! The program itself is replaced last.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The staging directory, in the base.
pub const STAGE: &str = ".baylee-update";
/// The unpacked new tree, in [`STAGE`].
pub const NEW: &str = "new";

/// A path relative to the base, one name per level.
pub type Rel = Vec<String>;

/// One step of an update.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rename {
    /// What is moved.
    pub from: Rel,
    /// Where to. Never exists before the step.
    pub to: Rel,
}

/// The three systems an update is installed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Os {
    /// `baylee-client.exe` in a folder.
    Windows,
    /// `Baylee.app`.
    MacOs,
    /// `baylee-client` in a folder.
    Linux,
}

impl Os {
    /// The system a target triple builds for, if it is one of the three.
    #[must_use]
    pub fn of_target(target: &str) -> Option<Self> {
        if target.contains("-windows-") {
            Some(Self::Windows)
        } else if target.contains("-apple-darwin") {
            Some(Self::MacOs)
        } else if target.contains("-linux-") && !target.contains("android") {
            Some(Self::Linux)
        } else {
            None
        }
    }

    /// What the archive calls the program, at the top of its tree.
    #[must_use]
    pub fn program(self) -> &'static str {
        match self {
            Self::Windows => "baylee-client.exe",
            Self::MacOs => "Baylee.app",
            Self::Linux => "baylee-client",
        }
    }

    /// Where the running program is set aside: on Windows the name the
    /// owner's design gives it, elsewhere the common rule.
    #[must_use]
    pub fn aside(self, name: &str) -> String {
        match self {
            Self::Windows if name.eq_ignore_ascii_case("baylee-client.exe") => {
                "baylee-client.old.exe".to_owned()
            }
            _ => format!("{name}.old"),
        }
    }
}

/// Why no plan was made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanError {
    /// The new tree has no program for this system: the wrong archive.
    NoProgram,
    /// Something already has the name an old entry would be set aside as,
    /// and it is not known to be ours.
    AsideTaken(String),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoProgram => f.write_str("the new tree has no program for this system"),
            Self::AsideTaken(name) => write!(f, "{name} is in the way"),
        }
    }
}

impl std::error::Error for PlanError {}

/// The renames that replace an installation.
///
/// `new` lists the top-level names of the staged tree, `existing` those in
/// the base, and `installed` the name the program has there (a player may
/// have renamed `Baylee.app`; the new bundle takes the old one's name).
///
/// # Errors
///
/// [`PlanError::NoProgram`] when `new` lacks this system's program, and
/// [`PlanError::AsideTaken`] when an old entry's aside name is taken.
pub fn plan(
    os: Os,
    new: &BTreeSet<String>,
    existing: &BTreeSet<String>,
    installed: &str,
) -> Result<Vec<Rename>, PlanError> {
    let program = os.program();
    if !new.contains(program) {
        return Err(PlanError::NoProgram);
    }
    let mut entries: Vec<(&str, &str)> = new
        .iter()
        .map(String::as_str)
        .filter(|name| *name != program && *name != STAGE)
        .filter(|name| os != Os::MacOs || existing.contains(*name))
        .map(|name| (name, name))
        .collect();
    entries.push((program, installed));
    let mut steps = Vec::with_capacity(entries.len() * 2);
    for (staged, here) in entries {
        if existing.contains(here) {
            let aside = os.aside(here);
            if existing.contains(&aside) {
                return Err(PlanError::AsideTaken(aside));
            }
            steps.push(Rename {
                from: vec![here.to_owned()],
                to: vec![aside],
            });
        }
        steps.push(Rename {
            from: vec![STAGE.to_owned(), NEW.to_owned(), staged.to_owned()],
            to: vec![here.to_owned()],
        });
    }
    Ok(steps)
}

/// The names [`plan`] sets old entries aside as, to delete once the update
/// has finished.
#[must_use]
pub fn asides(steps: &[Rename]) -> Vec<Rel> {
    steps
        .iter()
        .filter(|step| step.from.len() == 1)
        .map(|step| step.to.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    fn rel(parts: &[&str]) -> Rel {
        parts.iter().map(|s| (*s).to_owned()).collect()
    }

    fn step(from: &[&str], to: &[&str]) -> Rename {
        Rename {
            from: rel(from),
            to: rel(to),
        }
    }

    /// What the Windows archive holds (`package-client.sh`'s non-Apple
    /// arm): the program, its fonts, its line tables, the licences.
    const WINDOWS_ARCHIVE: &[&str] = &[
        "LICENSE",
        "NOTICE",
        "README.txt",
        "assets",
        "baylee-client.exe",
        "baylee_client.pdb",
    ];

    #[test]
    fn windows_sets_the_running_exe_aside_and_moves_everything_in() {
        let new = names(WINDOWS_ARCHIVE);
        let existing = names(WINDOWS_ARCHIVE);
        let steps = plan(Os::Windows, &new, &existing, "baylee-client.exe").unwrap();
        assert_eq!(
            steps,
            vec![
                step(&["LICENSE"], &["LICENSE.old"]),
                step(&[STAGE, NEW, "LICENSE"], &["LICENSE"]),
                step(&["NOTICE"], &["NOTICE.old"]),
                step(&[STAGE, NEW, "NOTICE"], &["NOTICE"]),
                step(&["README.txt"], &["README.txt.old"]),
                step(&[STAGE, NEW, "README.txt"], &["README.txt"]),
                step(&["assets"], &["assets.old"]),
                step(&[STAGE, NEW, "assets"], &["assets"]),
                step(&["baylee_client.pdb"], &["baylee_client.pdb.old"]),
                step(&[STAGE, NEW, "baylee_client.pdb"], &["baylee_client.pdb"]),
                step(&["baylee-client.exe"], &["baylee-client.old.exe"]),
                step(&[STAGE, NEW, "baylee-client.exe"], &["baylee-client.exe"]),
            ]
        );
    }

    /// What the new archive has that the installation lacks moves in with
    /// nothing set aside; what only the installation has stays.
    #[test]
    fn linux_adds_what_is_new_and_keeps_what_the_player_added() {
        let new = names(&["LICENSE", "assets", "baylee-client"]);
        let existing = names(&["assets", "baylee-client", "my-notes.txt"]);
        let steps = plan(Os::Linux, &new, &existing, "baylee-client").unwrap();
        assert_eq!(
            steps,
            vec![
                step(&[STAGE, NEW, "LICENSE"], &["LICENSE"]),
                step(&["assets"], &["assets.old"]),
                step(&[STAGE, NEW, "assets"], &["assets"]),
                step(&["baylee-client"], &["baylee-client.old"]),
                step(&[STAGE, NEW, "baylee-client"], &["baylee-client"]),
            ]
        );
        assert!(!steps.iter().any(|s| s.from == rel(&["my-notes.txt"])));
    }

    /// The unpacked folder: the bundle with the licences and symbols beside
    /// it. All of them are replaced.
    #[test]
    fn macos_in_its_unpacked_folder_replaces_the_bundle_and_its_neighbours() {
        let new = names(&[
            "Baylee.app",
            "LICENSE",
            "NOTICE",
            "README.txt",
            "baylee-client.dSYM",
        ]);
        let existing = names(&["Baylee.app", "LICENSE", "NOTICE", "README.txt"]);
        let steps = plan(Os::MacOs, &new, &existing, "Baylee.app").unwrap();
        assert_eq!(
            steps,
            vec![
                step(&["LICENSE"], &["LICENSE.old"]),
                step(&[STAGE, NEW, "LICENSE"], &["LICENSE"]),
                step(&["NOTICE"], &["NOTICE.old"]),
                step(&[STAGE, NEW, "NOTICE"], &["NOTICE"]),
                step(&["README.txt"], &["README.txt.old"]),
                step(&[STAGE, NEW, "README.txt"], &["README.txt"]),
                step(&["Baylee.app"], &["Baylee.app.old"]),
                step(&[STAGE, NEW, "Baylee.app"], &["Baylee.app"]),
            ]
        );
    }

    /// `/Applications`: the bundle alone, and nobody else's files are
    /// touched or joined by a `README.txt`.
    #[test]
    fn macos_in_applications_replaces_the_bundle_alone() {
        let new = names(&["Baylee.app", "LICENSE", "NOTICE", "README.txt"]);
        let existing = names(&["Baylee.app", "Safari.app", "Utilities"]);
        let steps = plan(Os::MacOs, &new, &existing, "Baylee.app").unwrap();
        assert_eq!(
            steps,
            vec![
                step(&["Baylee.app"], &["Baylee.app.old"]),
                step(&[STAGE, NEW, "Baylee.app"], &["Baylee.app"]),
            ]
        );
    }

    #[test]
    fn a_renamed_bundle_keeps_its_name() {
        let new = names(&["Baylee.app"]);
        let existing = names(&["Baylee Beta.app"]);
        let steps = plan(Os::MacOs, &new, &existing, "Baylee Beta.app").unwrap();
        assert_eq!(
            steps,
            vec![
                step(&["Baylee Beta.app"], &["Baylee Beta.app.old"]),
                step(&[STAGE, NEW, "Baylee.app"], &["Baylee Beta.app"]),
            ]
        );
    }

    #[test]
    fn another_systems_archive_is_refused() {
        let linux = names(&["assets", "baylee-client"]);
        for os in [Os::Windows, Os::MacOs] {
            assert_eq!(
                plan(os, &linux, &linux, os.program()),
                Err(PlanError::NoProgram),
                "{os:?}"
            );
        }
        let windows = names(WINDOWS_ARCHIVE);
        assert_eq!(
            plan(Os::Linux, &windows, &windows, "baylee-client"),
            Err(PlanError::NoProgram)
        );
    }

    #[test]
    fn a_taken_aside_is_refused() {
        let new = names(&["assets", "baylee-client.exe"]);
        let existing = names(&["assets", "baylee-client.exe", "baylee-client.old.exe"]);
        assert_eq!(
            plan(Os::Windows, &new, &existing, "baylee-client.exe"),
            Err(PlanError::AsideTaken("baylee-client.old.exe".into()))
        );
    }

    /// The staging directory is never a thing to install, even if an
    /// archive carried one.
    #[test]
    fn the_stage_is_never_moved() {
        let new = names(&[STAGE, "baylee-client"]);
        let existing = names(&[STAGE, "baylee-client"]);
        let steps = plan(Os::Linux, &new, &existing, "baylee-client").unwrap();
        assert!(
            steps
                .iter()
                .all(|s| s.from.first().map(String::as_str) != Some(STAGE) || s.from.len() == 3)
        );
        assert!(steps.iter().all(|s| s.to != rel(&[STAGE])));
    }

    #[test]
    fn every_step_is_set_aside_then_move_in_and_the_program_is_last() {
        for os in [Os::Windows, Os::MacOs, Os::Linux] {
            let new = names(&[os.program(), "LICENSE", "assets"]);
            let steps = plan(os, &new, &new, os.program()).unwrap();
            let last = steps.last().unwrap();
            assert_eq!(last.to, rel(&[os.program()]), "{os:?}");
            // Nothing is moved onto a name that is still occupied: each
            // move-in follows the set-aside of the same name.
            for pair in steps.chunks(2) {
                assert_eq!(pair[0].from, pair[1].to, "{os:?}");
            }
            assert_eq!(asides(&steps).len(), steps.len() / 2);
        }
    }

    #[test]
    fn the_system_is_read_off_the_target() {
        assert_eq!(Os::of_target("x86_64-pc-windows-msvc"), Some(Os::Windows));
        assert_eq!(Os::of_target("aarch64-pc-windows-msvc"), Some(Os::Windows));
        assert_eq!(Os::of_target("aarch64-apple-darwin"), Some(Os::MacOs));
        assert_eq!(Os::of_target("x86_64-unknown-linux-gnu"), Some(Os::Linux));
        assert_eq!(Os::of_target("aarch64-unknown-linux-gnu"), Some(Os::Linux));
        assert_eq!(Os::of_target("aarch64-linux-android"), None);
        assert_eq!(Os::of_target("aarch64-apple-ios"), None);
        assert_eq!(Os::of_target("wasm32-unknown-unknown"), None);
    }
}
