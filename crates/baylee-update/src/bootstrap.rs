//! The first stage of the desktop launcher: what stays of it when an update cannot
//! reach it.
//!
//! The executable in a package is never replaced (`docs/releasing.md` §"Desktop
//! launcher and recovery"), so whatever it does is frozen for as long as that
//! package is installed. So it does as little as it can: it reads the pointer,
//! checks it, and starts the launcher of the selected generation, which came with
//! that release and is as new as the client. Everything else happens in there. The
//! same binary is both stages, and [`ENV`] says which one it is.
//!
//! **Frozen with it:** [`ENV`] and its record, [`ATTEMPTS`] and its record, and where
//! a generation keeps its launcher. A later launcher has to read what a package
//! built today writes.
//!
//! The pointer is the one thing read here, and it is a commit: `current.json` is
//! replaced atomically, and only after the whole tree stands in `versions/<id>`. An
//! activation cut short therefore leaves it on the previous generation, whose
//! launcher finishes that activation as it starts. So `activation.json` stays in
//! the updatable stage, and this stage holds no lock.
//!
//! A line is fixed when it is compiled: a generation comes from its own line's
//! archive, and nothing here asks the CPU.

use crate::apply::{self, Install};
use crate::launch;
use crate::plan::Os;
use crate::relaunch;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use uuid::Uuid;

/// Set by the first stage for the second: the package this launcher starts for.
pub(crate) const ENV: &str = "BAYLEE_LAUNCH_PACKAGE";

/// How often a generation's launcher was started without a client joining.
const ATTEMPTS: &str = "launch-attempts.json";

/// Starts a generation's launcher may fail before the package's own logic takes over.
const MOST_ATTEMPTS: u32 = 2;

/// What the second stage cannot know: it runs from `versions/<id>`, and without
/// this it would key a state of its own and take its own version for the package's.
#[derive(Serialize, Deserialize)]
struct Handed {
    install: Install,
    version: String,
}

#[derive(Serialize, Deserialize)]
struct Attempts {
    generation: Uuid,
    count: u32,
}

/// Starts a client through the launcher at `exe`, as either stage.
///
/// A record that cannot be read is as good as none: this launcher then starts for
/// the package around it, which is what a stray variable should cost.
///
/// # Errors
/// As [`launch::run`], and a launcher outside any package layout.
pub fn start(exe: &Path, os: Os, args: Vec<OsString>) -> io::Result<ExitStatus> {
    if let Some((install, version)) = handed() {
        return launch::run(&install, &version, true, args);
    }
    let install = Install::around_launcher(exe, os).map_err(io::Error::other)?;
    let version = launch::package_version();
    match hand_on(&install, &version, &args) {
        Some(status) => Ok(status),
        None => launch::run(&install, &version, false, args),
    }
}

fn handed() -> Option<(Install, Version)> {
    let record = std::env::var_os(ENV)?;
    let handed: Handed = serde_json::from_str(record.to_str()?).ok()?;
    Some((handed.install, Version::parse(&handed.version).ok()?))
}

/// Hands this start to the selected generation's launcher, and waits for it. `None`
/// when this package's own logic should start the client, which nothing here has
/// touched: the arguments are whole and stdin is unread.
///
/// Stdin is inherited, as `launch::run` leaves it for the runtime: the handoff of
/// "Restart to update" crosses here on it, and it is a secret
/// (`crate::relaunch`). It is neither read here nor put anywhere else.
fn hand_on(install: &Install, version: &Version, args: &[OsString]) -> Option<ExitStatus> {
    let (install, _) = launch::placed(install).ok()?;
    let generation = launch::newer_generation(&install, version)?;
    let launcher = launcher_of(&install, generation)?;
    note_the_attempt(&install, generation)?;
    let record = serde_json::to_string(&Handed {
        install,
        version: version.to_string(),
    })
    .ok()?;
    let mut command = Command::new(launcher);
    command.args(args).env(ENV, record);
    relaunch::no_window(&mut command);
    command.spawn().ok()?.wait().ok()
}

/// The launcher of `generation`, resolved, and only if it lies in that generation's
/// own folder of this installation's state. `generation` came out of a file, but as
/// a [`Uuid`] it is no path; a link in `versions` could still lead out of it.
fn launcher_of(install: &Install, generation: Uuid) -> Option<PathBuf> {
    let folder = Path::new("versions").join(generation.to_string());
    let file = match install.os {
        Os::MacOs => Path::new(install.os.program())
            .join("Contents/MacOS")
            .join(relaunch::MAC_LAUNCHER),
        Os::Windows | Os::Linux => PathBuf::from(install.os.program()),
    };
    let stage = fs::canonicalize(install.stage()).ok()?;
    let launcher = fs::canonicalize(stage.join(&folder).join(file)).ok()?;
    (launcher.starts_with(stage.join(folder)) && launcher.is_file()).then_some(launcher)
}

/// Writes down this attempt before it is made: a launcher that dies leaves nobody to
/// say so. `None` when this generation has had its attempts, or when the attempt
/// could not be written and so could not be counted.
fn note_the_attempt(install: &Install, generation: Uuid) -> Option<()> {
    let path = install.stage().join(ATTEMPTS);
    let count = apply::read_json::<Attempts>(&path)
        .filter(|attempts| attempts.generation == generation)
        .map_or(0, |attempts| attempts.count);
    if count >= MOST_ATTEMPTS {
        return None;
    }
    let count = count + 1;
    apply::write_json(&path, &Attempts { generation, count }).ok()
}

/// The second stage did its work: a client joined it, or it reached the lock and
/// found one running. Only a start it was handed may say so, or a generation whose
/// launcher fails would get its attempts back from the package's own logic.
pub(crate) fn went_well(install: &Install) {
    let _ = fs::remove_file(install.stage().join(ATTEMPTS));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(tag: &str) -> Install {
        let state = std::env::temp_dir().join(format!("baylee-boot-{tag}-{}", Uuid::now_v7()));
        fs::create_dir_all(&state).unwrap();
        let install = Install {
            state: Some(state),
            os: Os::Linux,
            base: PathBuf::from("/nowhere"),
            program: "baylee-client".into(),
        };
        fs::create_dir_all(install.stage().join("versions")).unwrap();
        install
    }

    fn a_generation(install: &Install) -> Uuid {
        let generation = Uuid::now_v7();
        let folder = install
            .stage()
            .join("versions")
            .join(generation.to_string());
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("baylee-client"), "a launcher").unwrap();
        generation
    }

    #[test]
    fn a_generations_own_launcher_is_found_and_a_missing_one_is_not() {
        let install = at("found");
        let generation = a_generation(&install);
        let launcher = launcher_of(&install, generation).expect("its launcher");
        assert!(
            launcher.ends_with(
                Path::new("versions")
                    .join(generation.to_string())
                    .join("baylee-client")
            ),
            "{}",
            launcher.display()
        );
        assert_eq!(launcher_of(&install, Uuid::now_v7()), None);
    }

    /// A folder where the launcher should be is not a launcher.
    #[test]
    fn a_folder_in_the_launchers_place_starts_nothing() {
        let install = at("folder");
        let generation = Uuid::now_v7();
        let place = install
            .stage()
            .join("versions")
            .join(generation.to_string())
            .join("baylee-client");
        fs::create_dir_all(&place).unwrap();
        assert_eq!(launcher_of(&install, generation), None);
    }

    /// A pointer is a value out of a file: what is no UUID selects nothing here, and
    /// the package's own logic reports it.
    #[test]
    fn a_pointer_that_names_a_path_starts_nothing() {
        let install = at("path");
        for generation in ["../../elsewhere", "..", "versions/x", ""] {
            let pointer = serde_json::json!({
                "generation": generation, "from": "1.0.0", "to": "9.0.0", "announced": true,
            });
            fs::write(install.stage().join("current.json"), pointer.to_string()).unwrap();
            let original = Version::new(1, 0, 0);
            assert_eq!(launch::newer_generation(&install, &original), None);
        }
    }

    /// A version that does not parse, or one no newer than the package, hands
    /// nothing on: the package's own logic decides, as it always did.
    #[test]
    fn only_a_newer_parsable_version_is_handed_on() {
        let install = at("versions");
        let generation = Uuid::now_v7();
        let original = Version::new(1, 0, 0);
        for (to, handed) in [
            ("x.y", false),
            ("1.0.0", false),
            ("0.9.0", false),
            ("1.0.1", true),
        ] {
            let pointer = serde_json::json!({
                "generation": generation, "from": "1.0.0", "to": to, "announced": true,
            });
            fs::write(install.stage().join("current.json"), pointer.to_string()).unwrap();
            assert_eq!(
                launch::newer_generation(&install, &original).is_some(),
                handed,
                "{to}"
            );
        }
    }

    /// Both ways out of the generation's folder: the folder itself a link, and the
    /// launcher in it one. Making a link may need a right Windows did not give.
    #[test]
    fn a_link_out_of_the_generation_starts_nothing() {
        #[cfg(unix)]
        use std::os::unix::fs::{symlink as dir_link, symlink as file_link};
        #[cfg(windows)]
        use std::os::windows::fs::{symlink_dir as dir_link, symlink_file as file_link};
        let install = at("link");
        let outside = std::env::temp_dir().join(format!("baylee-outside-{}", Uuid::now_v7()));
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("baylee-client"), "not ours").unwrap();

        let generation = Uuid::now_v7();
        let folder = install
            .stage()
            .join("versions")
            .join(generation.to_string());
        if dir_link(&outside, &folder).is_err() {
            eprintln!("this user may not make links: skipped");
            return;
        }
        assert_eq!(launcher_of(&install, generation), None, "the folder");

        let generation = a_generation(&install);
        let folder = install
            .stage()
            .join("versions")
            .join(generation.to_string());
        fs::remove_file(folder.join("baylee-client")).unwrap();
        file_link(outside.join("baylee-client"), folder.join("baylee-client")).unwrap();
        assert_eq!(launcher_of(&install, generation), None, "the launcher");
    }

    #[test]
    fn a_generation_has_two_attempts_and_the_next_one_two_more() {
        let install = at("attempts");
        let (first, second) = (Uuid::now_v7(), Uuid::now_v7());
        assert!(note_the_attempt(&install, first).is_some());
        assert!(note_the_attempt(&install, first).is_some());
        assert!(note_the_attempt(&install, first).is_none());
        assert!(
            note_the_attempt(&install, first).is_none(),
            "and it stays so"
        );
        assert!(note_the_attempt(&install, second).is_some());
        went_well(&install);
        assert!(note_the_attempt(&install, second).is_some());
        assert!(note_the_attempt(&install, second).is_some());
    }

    /// An attempt that cannot be written cannot be counted, so it is not made.
    #[test]
    fn an_attempt_that_cannot_be_written_is_not_made() {
        let install = at("unwritable");
        let temporary = install.stage().join(ATTEMPTS).with_extension("json.tmp");
        fs::create_dir_all(temporary).unwrap();
        assert!(note_the_attempt(&install, Uuid::now_v7()).is_none());
    }

    /// A start that went well clears the count, and clearing a count that is not
    /// there is no error.
    #[test]
    fn going_well_clears_the_count_and_a_missing_one_is_fine() {
        let install = at("cleared");
        went_well(&install);
        assert!(note_the_attempt(&install, Uuid::now_v7()).is_some());
        assert!(install.stage().join(ATTEMPTS).exists());
        went_well(&install);
        assert!(!install.stage().join(ATTEMPTS).exists());
    }

    #[test]
    fn a_torn_count_starts_over_rather_than_locking_anyone_out() {
        let install = at("torn");
        fs::write(install.stage().join(ATTEMPTS), "{").unwrap();
        assert!(note_the_attempt(&install, Uuid::now_v7()).is_some());
    }
}
