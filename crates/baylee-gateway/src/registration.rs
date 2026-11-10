//! Who may come in new (`BAYLEE_REGISTRATION`): anybody, or nobody.
//!
//! The closed beta's keys (#317) are gone with the open beta (owner,
//! 10.11.2026), and with them the third answer, `invite`. A gateway still
//! set to it refuses to start rather than guess which of the two left its
//! operator meant: read as `open` it would let in the strangers the key kept
//! out, and read as `off` it would keep out the players the open beta is
//! for. Either guess would be made in silence; the refusal names the fix.
//!
//! `baylee-gateway invite …`, the command that made the keys, is answered
//! the same way ([`INVITE_COMMAND_GONE`]) instead of falling through to
//! starting a gateway with whatever settings the shell held.

/// Who may make an account here (`BAYLEE_REGISTRATION`).
///
/// @description Who may come in new: `open` (anybody) or `off` (nobody).
/// A gateway from before the open beta may still say `invite`.
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Registration {
    /// Anybody.
    Open,
    /// Nobody.
    Off,
}

/// Why `BAYLEE_REGISTRATION=invite` stops the start.
pub const INVITE_GONE: &str = "`invite` was the closed beta's, and its keys are gone: set \
     BAYLEE_REGISTRATION=open to let new players in (the open beta), or off to let nobody new in";

/// What `baylee-gateway invite …` answers since the keys are gone.
pub const INVITE_COMMAND_GONE: &str = "baylee-gateway invite: the closed beta's keys are gone \
     (the beta is open); there is nothing to make, list or revoke. BAYLEE_REGISTRATION=open \
     lets new players in, off lets nobody new in";

impl Registration {
    /// `BAYLEE_REGISTRATION` read: the three spellings `switched_on` reads
    /// as off are [`Self::Off`], `invite` is refused ([`INVITE_GONE`]), and
    /// anything else, unset included, leaves registration open as it always
    /// was.
    ///
    /// Exactly `invite` is refused and nothing like it, for the reason
    /// `switched_on` gives: widening what a switch accepts and refusing what
    /// it does not know are decisions of their own. `invite` is refused
    /// because it was a word this gateway knew and gave a meaning neither
    /// remaining answer has.
    ///
    /// # Errors
    ///
    /// [`INVITE_GONE`] for `invite`.
    pub fn from_env(raw: Option<&str>) -> Result<Self, &'static str> {
        match raw {
            Some("invite") => Err(INVITE_GONE),
            Some("off" | "0" | "false") => Ok(Self::Off),
            _ => Ok(Self::Open),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_reads_two_ways_and_refuses_the_closed_beta() {
        for off in ["off", "0", "false"] {
            assert_eq!(Registration::from_env(Some(off)), Ok(Registration::Off));
        }
        assert_eq!(Registration::from_env(Some("invite")), Err(INVITE_GONE));
        assert!(INVITE_GONE.contains("BAYLEE_REGISTRATION=open"));
        // What `switched_on` reads as on stays open, however near it is;
        // only the one word the closed beta used is refused.
        for open in [
            None,
            Some(""),
            Some("on"),
            Some("open"),
            Some("Invite"),
            Some("OFF"),
            Some("no"),
        ] {
            assert_eq!(
                Registration::from_env(open),
                Ok(Registration::Open),
                "{open:?}"
            );
        }
        // How `GET /info` and `GET /auth/config` spell it.
        assert_eq!(
            [Registration::Open, Registration::Off].map(|r| serde_json::to_value(r).unwrap()),
            ["open", "off"]
        );
    }
}
