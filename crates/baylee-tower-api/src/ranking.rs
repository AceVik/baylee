//! Verified, cohort-local personal bests. Public participation is a separate opt-in.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A bounded podium plus the requesting player's own best, even outside the podium.
pub const LEADERS: usize = 25;

/// Any house fallback puts the entire completed run in the separate house board.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lane {
    /// Every verified opponent was a same-stage historical player echo.
    #[default]
    Echoes,
    /// At least one verified opponent was a clearly labelled house fallback.
    House,
}

/// Independently revisioned, reversible public leaderboard participation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Listing {
    /// Compare-and-set fence; retries cannot undo a later privacy choice.
    pub revision: u64,
    /// Explicit permission to publish the verified public name and personal bests.
    pub listed: bool,
}

/// Confirmed Central public identity, never a client-supplied alias or login name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Name {
    /// Public display name, copied only after live identity introspection.
    pub display: String,
    /// Public discriminator; not an account UUID or credential.
    pub tag: u32,
}

impl Name {
    /// Defense in depth at both transport boundaries; text is rendered literally.
    #[must_use]
    pub fn valid(&self) -> bool {
        let bytes = self.display.as_bytes();
        (3..=16).contains(&bytes.len())
            && bytes.first().is_some_and(u8::is_ascii_alphanumeric)
            && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
            && bytes
                .iter()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            && (1..=0x7fff_ffff).contains(&self.tag)
    }
}

/// One account's best completed verified run, with no timing/wealth tiebreaker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Standing {
    /// Competition rank (1, 1, 3). Private personal bests have no public rank.
    pub rank: Option<u64>,
    /// Confirmed public name, absent only for an unlisted personal best.
    pub name: Option<Name>,
    /// Higher wins rank first; always at most eight.
    pub wins: u8,
    /// Defeats and rules draws; fewer rank first among equal wins.
    pub defeats: u8,
    /// Binds an entry to the authenticated viewer without publishing account IDs.
    pub yours: bool,
}

/// One immutable compatibility cohort; scores never cross rules or AI revisions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Board {
    /// Published cohort selected by the caller.
    pub cohort: Uuid,
    /// Separate all-echo or house-assisted comparison.
    pub lane: Lane,
    /// This account's confirmed publication preference.
    pub listing: Listing,
    /// This account's best, including while private or outside the top 25.
    pub best: Option<Standing>,
    /// Up to [`LEADERS`] public personal bests, ordered by wins then defeats.
    pub leaders: Vec<Standing>,
}

impl Board {
    /// Validate bounded scores, identity text, ordering and exact shared podium ranks.
    #[must_use]
    pub fn valid(&self) -> bool {
        let valid = |s: &Standing| {
            ((s.wins == 8 && s.defeats < 5) || (s.wins < 8 && s.defeats == 5))
                && s.rank != Some(0)
                && s.name.as_ref().is_none_or(Name::valid)
                && (s.rank.is_some() == s.name.is_some())
        };
        if self.leaders.len() > LEADERS
            || self
                .best
                .as_ref()
                .is_some_and(|s| !valid(s) || !s.yours || s.rank.is_some() != self.listing.listed)
        {
            return false;
        }
        let mut previous: Option<&Standing> = None;
        for (index, entry) in self.leaders.iter().enumerate() {
            let same = previous.is_some_and(|p| (p.wins, p.defeats) == (entry.wins, entry.defeats));
            let rank = if same {
                previous.and_then(|p| p.rank)
            } else {
                Some(index as u64 + 1)
            };
            if !valid(entry)
                || entry.name.is_none()
                || entry.rank != rank
                || previous.is_some_and(|p| {
                    (std::cmp::Reverse(p.wins), p.defeats)
                        > (std::cmp::Reverse(entry.wins), entry.defeats)
                })
                || (entry.yours && self.best.as_ref() != Some(entry))
            {
                return false;
            }
            previous = Some(entry);
        }
        self.leaders.iter().filter(|s| s.yours).count() <= 1
    }
}
