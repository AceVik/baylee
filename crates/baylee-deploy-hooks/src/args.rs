//! The dispatcher's two arguments, checked before anything else happens.
//!
//! `run-deploy-hooks PHASE COMMIT`: a phase from [`Phase`] and a full commit
//! id, exactly 40 lowercase hexadecimal digits. Anything else (a third
//! argument, an option, an abbreviated or upper-case id) is a usage error and
//! runs nothing.

use std::ffi::OsString;
use std::fmt;
use std::time::Duration;

/// When in a deploy the hooks run (`docs/deploy-hooks.md` §"The phases").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Before the deploy changes anything installed or running.
    Prepare,
    /// After the games are drained, before the gateway is replaced.
    BeforeSwitch,
    /// After the new gateway answers with the target commit, before games
    /// are admitted again.
    AfterSwitch,
}

impl Phase {
    /// Every phase, in the order a deploy runs them.
    pub const ALL: [Self; 3] = [Self::Prepare, Self::BeforeSwitch, Self::AfterSwitch];

    /// The word a hook is called with.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Prepare => "prepare",
            Self::BeforeSwitch => "before-switch",
            Self::AfterSwitch => "after-switch",
        }
    }

    /// How long all of a phase's hooks together may take.
    ///
    /// `prepare` may fetch and verify artifacts; the other two run while no
    /// game can start, so they are short.
    #[must_use]
    pub fn budget(self) -> Duration {
        Duration::from_secs(match self {
            Self::Prepare => 900,
            Self::BeforeSwitch => 180,
            Self::AfterSwitch => 120,
        })
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

/// Why the arguments were refused.
#[derive(Debug, PartialEq, Eq)]
pub enum Usage {
    /// Not exactly two arguments.
    Count,
    /// The first is not a phase.
    Phase,
    /// The second is not 40 lowercase hexadecimal digits.
    Commit,
}

impl fmt::Display for Usage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Count => "exactly two arguments",
            Self::Phase => "the phase is prepare, before-switch or after-switch",
            Self::Commit => "the commit is 40 lowercase hexadecimal digits",
        })
    }
}

/// What the binary says on a usage error.
pub const USAGE: &str =
    "usage: run-deploy-hooks prepare|before-switch|after-switch <40-hex commit>";

/// The phase and the commit, or why not.
///
/// # Errors
/// [`Usage`] for anything but exactly a phase word and a full lowercase
/// commit id.
pub fn parse(args: &[OsString]) -> Result<(Phase, String), Usage> {
    let [phase, commit] = args else {
        return Err(Usage::Count);
    };
    let phase = Phase::ALL
        .into_iter()
        .find(|p| phase.to_str() == Some(p.word()))
        .ok_or(Usage::Phase)?;
    let commit = commit.to_str().ok_or(Usage::Commit)?;
    if commit.len() != 40
        || !commit
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Usage::Commit);
    }
    Ok((phase, commit.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_phase_and_a_full_commit_are_taken() {
        for phase in Phase::ALL {
            assert_eq!(
                parse(&args(&[phase.word(), SHA])),
                Ok((phase, SHA.to_owned()))
            );
        }
    }

    #[test]
    fn anything_else_is_refused() {
        let upper = SHA.to_uppercase();
        let short = &SHA[..39];
        let long = format!("{SHA}0");
        let cases: [(&[&str], Usage); 11] = [
            (&[], Usage::Count),
            (&["prepare"], Usage::Count),
            (&["prepare", SHA, "x"], Usage::Count),
            (&["--help", SHA], Usage::Phase),
            (&["Prepare", SHA], Usage::Phase),
            (&["prepare ", SHA], Usage::Phase),
            (&["switch", SHA], Usage::Phase),
            (&["prepare", &upper], Usage::Commit),
            (&["prepare", short], Usage::Commit),
            (&["prepare", &long], Usage::Commit),
            (
                &["prepare", "0123456789abcdef0123456789abcdef0123456g"],
                Usage::Commit,
            ),
        ];
        for (given, why) in cases {
            assert_eq!(parse(&args(given)), Err(why), "{given:?}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_commit_that_is_not_utf8_is_refused() {
        use std::os::unix::ffi::OsStringExt as _;
        let bad = OsString::from_vec(vec![0xff; 40]);
        assert_eq!(parse(&[OsString::from("prepare"), bad]), Err(Usage::Commit));
    }

    #[test]
    fn the_budgets_are_the_documented_ones() {
        assert_eq!(Phase::Prepare.budget(), Duration::from_mins(15));
        assert_eq!(Phase::BeforeSwitch.budget(), Duration::from_secs(180));
        assert_eq!(Phase::AfterSwitch.budget(), Duration::from_secs(120));
    }
}
