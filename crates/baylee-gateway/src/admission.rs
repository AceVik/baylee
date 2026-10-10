//! The admission hold (`BAYLEE_ADMISSION_HOLD`): a file whose existence
//! stops every new game, whichever agent would run it.
//!
//! A server deploy with deploy hooks (`docs/deploy-hooks.md`) has to wait
//! until no game is running anywhere before it replaces the gateway, and
//! stopping this machine's agent does not stop an agent elsewhere from being
//! ordered a new one. So the deployer (as root) creates the file, and while
//! it exists [`crate::engine::start_engine`] orders no engine: a new game, a
//! room's start and a rematch all answer `503`. Games already running go
//! on. `/health` says `admission`: `held`, `open`, or `unmanaged` when no
//! path is set, which is how the deployer checks that the gateway it talks
//! to honours the file before it relies on it.
//!
//! Read on every start rather than watched: one `stat` per game started
//! costs nothing, and a file survives the gateway's own restart, which is
//! what keeps the new gateway from admitting games before the deploy is
//! done. Anything but "not found" (a permission error, say) counts as held:
//! a hold that cannot be read is not a reason to start a game.

use std::path::PathBuf;

/// What `/health` says about admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Admission {
    /// No `BAYLEE_ADMISSION_HOLD`: nothing here can hold games.
    Unmanaged,
    /// The hold file is absent: games start.
    Open,
    /// The hold file is there: no new game starts.
    Held,
}

impl Admission {
    /// The word `/health` answers.
    pub(crate) fn wire(self) -> &'static str {
        match self {
            Self::Unmanaged => "unmanaged",
            Self::Open => "open",
            Self::Held => "held",
        }
    }
}

/// What a refused start says, and the reason `start_engine` returns.
pub(crate) const HELD: &str =
    "the server is being updated: no new game starts until it is back, in a few minutes";

/// Where the hold file is, if anywhere.
#[derive(Debug, Default)]
pub(crate) struct Hold {
    path: Option<PathBuf>,
}

impl Hold {
    /// From `BAYLEE_ADMISSION_HOLD`: unset or blank is no hold; anything
    /// else must be an absolute path.
    pub(crate) fn from_env(raw: Option<&str>) -> Result<Self, String> {
        let Some(raw) = raw.map(str::trim).filter(|raw| !raw.is_empty()) else {
            return Ok(Self::default());
        };
        let path = PathBuf::from(raw);
        if !path.is_absolute() {
            return Err(format!("{raw} is not an absolute path"));
        }
        Ok(Self { path: Some(path) })
    }

    /// Whether games may start now.
    pub(crate) fn admission(&self) -> Admission {
        let Some(path) = &self.path else {
            return Admission::Unmanaged;
        };
        match std::fs::symlink_metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Admission::Open,
            _ => Admission::Held,
        }
    }

    /// Whether no new game may start.
    pub(crate) fn held(&self) -> bool {
        self.admission() == Admission::Held
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_or_blank_holds_nothing_and_a_relative_path_is_refused() {
        for raw in [None, Some(""), Some("  ")] {
            let hold = Hold::from_env(raw).unwrap();
            assert_eq!(hold.admission(), Admission::Unmanaged);
            assert!(!hold.held());
        }
        assert!(Hold::from_env(Some("var/hold")).is_err());
    }

    #[test]
    fn the_file_holds_while_it_exists() {
        let path = std::env::temp_dir().join(format!("baylee-hold-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let hold = Hold::from_env(Some(&path.display().to_string())).unwrap();
        assert_eq!(hold.admission(), Admission::Open);
        std::fs::write(&path, b"").unwrap();
        assert_eq!(hold.admission(), Admission::Held);
        assert!(hold.held());
        std::fs::remove_file(&path).unwrap();
        assert_eq!(hold.admission(), Admission::Open);
    }

    #[cfg(unix)]
    #[test]
    fn a_hold_that_cannot_be_read_holds() {
        // A path through a file: `stat` answers "not a directory", not
        // "not found".
        let file = std::env::temp_dir().join(format!("baylee-hold-file-{}", std::process::id()));
        std::fs::write(&file, b"").unwrap();
        let hold = Hold::from_env(Some(&file.join("hold").display().to_string())).unwrap();
        assert_eq!(hold.admission(), Admission::Held);
        std::fs::remove_file(&file).unwrap();
    }
}
