//! Restarting into an update and coming back to exactly where the player
//! was (beta.6, the owner, 09.10.2026; `docs/client.md` §"Restarting into an
//! update").
//!
//! Two halves cross the restart, and only one of them touches the disk:
//!
//! - **The resume file** ([`ResumeState`], `resume.json` beside the
//!   settings, written readable by its owner alone): where the player was —
//!   the gateway, the lobby's screen, the deck being edited, the waiting
//!   room, the game (a hosted game's id and seat, or the house game's record
//!   file and the hash it had reached) and how the table was being looked
//!   at. **No secret**: no session, no seat token, no password.
//! - **The handoff** ([`Handoff`]): the file's nonce and the sign-in's
//!   session, which travel only through pipes, from the old client through
//!   the relaunch helper and the launcher to the new one
//!   (`baylee_update::relaunch`). A hosted seat's own secret never travels
//!   at all: the gateway hands the signed-in account a new one for its own
//!   chair (`POST /lobby/games/{id}/seat`).
//!
//! The file is taken ([`accept`]) only with a handoff carrying its nonce and
//! only while it is [`FRESH_FOR_MS`] old; anything else — no handoff, another
//! nonce, an older file, a file from a shape this build does not read — is
//! no resume, and the client starts as it always does. The file is deleted
//! as it is read, taken or not.

use serde::{Deserialize, Serialize};

use crate::settings_map::Section;
use crate::tableview::Arrangement;

/// The resume file's name beside the settings.
pub const RESUME_FILE: &str = "resume.json";

/// The argument a client started again is given: look for a resume file and
/// a handoff. Without it neither is read, and a leftover file is removed.
pub const RESUME_ARG: &str = "--resume";

/// How long a resume file holds: ten minutes, far longer than any restart
/// and short enough that a file left by a restart that never happened does
/// not put the player back at a table the next morning.
pub const FRESH_FOR_MS: u64 = 10 * 60 * 1000;

/// The file's shape; a client reads only its own.
pub const SHAPE: u32 = 1;

/// Where the player was, as the resume file holds it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResumeState {
    /// [`SHAPE`].
    pub shape: u32,
    /// Random, and the handoff carries the same: a file nobody handed over
    /// a nonce for is not this restart's.
    pub nonce: String,
    /// When it was written, milliseconds since the Unix epoch.
    pub written_at_ms: u64,
    /// The gateway the player had chosen, as the gateway list spells it;
    /// `None` on the gateway list and offline.
    pub gateway: Option<String>,
    /// Who was signed in there, without anything to sign in with: the
    /// session comes through the [`Handoff`].
    pub account: Option<Account>,
    /// The lobby's screen.
    pub front: Front,
    /// The game being played, if one was.
    pub game: Option<Game>,
    /// How the table was being looked at, if one was open.
    pub table: Option<TableLook>,
}

/// Who was signed in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// The username a registered account signed in with; `None` for a guest.
    pub username: Option<String>,
    /// A guest, whose session the settings already keep.
    pub guest: bool,
}

/// The lobby's screen, as far as a restart puts it back.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Front {
    /// Which face or screen.
    pub place: Place,
    /// The hub's tab: `true` for Decks, `false` for Play.
    #[serde(default)]
    pub decks_tab: bool,
    /// The settings section open over it, by its place in [`Section::ALL`].
    #[serde(default)]
    pub settings: Option<u8>,
    /// The deck open in the builder: its id, or `None` for a new deck.
    /// Its unsaved rows are the builder's own draft file's, which opening
    /// the same deck again restores.
    #[serde(default)]
    pub editing: Option<Option<String>>,
    /// The waiting room the player sat in, by game id.
    #[serde(default)]
    pub room: Option<String>,
    /// Playing offline (against the house), with no gateway.
    #[serde(default)]
    pub offline: bool,
}

/// The lobby's screens a restart can put back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    /// The gateway list, before one is chosen.
    #[default]
    Gateways,
    /// The sign-in face.
    SignIn,
    /// The hub (Play or Decks).
    Hub,
    /// The deck builder.
    Builder,
    /// At a table.
    Table,
}

/// The game being played.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Game {
    /// At a gateway: the table and the chair. The chair's ticket is asked
    /// for again by the signed-in account.
    Hosted {
        /// The game.
        game_id: String,
        /// This player's chair.
        seat: u32,
    },
    /// Against the house, in this client: rebuilt from its kept record.
    Local {
        /// The record's file among the kept records
        /// (`bugreport::record_file_name`).
        record: String,
        /// This player's chair.
        seat: u8,
        /// What each seat was called.
        names: Vec<String>,
        /// The engine's hash after the record's last input, as the record
        /// writes it: the rebuilt game must reach it.
        hash: String,
    },
}

/// How the table was being looked at.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableLook {
    /// This game's arrangement switch, when the player had set one.
    #[serde(default)]
    pub arrangement: Option<Arrangement>,
    /// The seat the camera was visiting.
    #[serde(default)]
    pub visiting: Option<u8>,
    /// The phone's hand drawer was open.
    #[serde(default)]
    pub hand_drawer_open: bool,
    /// The question's sheet was folded.
    #[serde(default)]
    pub sheet_folded: bool,
    /// The zone browser was open.
    #[serde(default)]
    pub browser_open: bool,
}

/// What travels through the pipes, never through a file.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handoff {
    /// The resume file's nonce.
    pub nonce: String,
    /// The session the player was signed in with, if any.
    #[serde(default)]
    pub session: Option<String>,
}

impl std::fmt::Debug for Handoff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handoff")
            .field("nonce", &self.nonce)
            .field("session", &self.session.as_ref().map(|_| "…"))
            .finish()
    }
}

impl Handoff {
    /// As the pipe carries it.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Read back off the pipe; `None` for anything else.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }
}

/// Why a resume file was not taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// It does not read as a resume file of this shape.
    Unreadable,
    /// No handoff, or one for another file.
    Foreign,
    /// Older than [`FRESH_FOR_MS`], or from the future.
    Stale,
    /// What it names cannot be right (a record file that is not one).
    Malformed,
}

/// The resume file `text` read with the `handoff` the restart brought, at
/// `now_ms`: where to go back to, or why not.
///
/// # Errors
/// [`Refused`].
pub fn accept(text: &str, handoff: Option<&Handoff>, now_ms: u64) -> Result<ResumeState, Refused> {
    let state: ResumeState = serde_json::from_str(text).map_err(|_| Refused::Unreadable)?;
    if state.shape != SHAPE {
        return Err(Refused::Unreadable);
    }
    let Some(handoff) = handoff else {
        return Err(Refused::Foreign);
    };
    if handoff.nonce.is_empty() || handoff.nonce != state.nonce {
        return Err(Refused::Foreign);
    }
    // A minute of a clock set back is forgiven; a file from further ahead
    // was not written by this restart.
    if state.written_at_ms > now_ms.saturating_add(60_000)
        || now_ms.saturating_sub(state.written_at_ms) > FRESH_FOR_MS
    {
        return Err(Refused::Stale);
    }
    if let Some(Game::Local { record, names, .. }) = &state.game
        && (!crate::bugreport::is_record_file_name(record) || names.len() > 8)
    {
        return Err(Refused::Malformed);
    }
    Ok(state)
}

/// A fresh nonce from 128 random bits, hex.
#[must_use]
pub fn nonce(random: [u8; 16]) -> String {
    random.iter().fold(String::with_capacity(32), |mut out, b| {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// A settings section as the file names it.
#[must_use]
pub fn section_number(section: Section) -> u8 {
    Section::ALL
        .iter()
        .position(|s| *s == section)
        .and_then(|at| u8::try_from(at).ok())
        .unwrap_or(0)
}

/// The section the file names, if it names one this build has.
#[must_use]
pub fn section_of(number: u8) -> Option<Section> {
    Section::ALL.get(usize::from(number)).copied()
}

/// When the "update ready — restart" offer may appear at a table.
///
/// Never in the middle of a decision of this player's: a panel appearing
/// beside the question being read draws the eye away from it. An update
/// that becomes ready while the player owes an answer is held until that
/// question is answered, and appears then — between that answer and
/// whatever comes next — or at once while nothing is asked. Once it stands
/// it stays (it is a panel, not a question, and the choice is the
/// player's) until "Later" (`put_off`) or there is nothing ready.
///
/// Against the house the player is asked almost every frame (each priority
/// is theirs), so "only while not asked" would never show it at all; the
/// moment after an answer is the one moment that interrupts nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RestartOffer {
    /// The question it arrived during, by the view's `seq`.
    held_at: Option<u64>,
    /// It stands.
    standing: bool,
}

impl RestartOffer {
    /// One frame: whether the offer stands now. `asked` is the `seq` of the
    /// question this player owes now, `None` while nothing is asked of them.
    pub fn step(&mut self, ready: bool, put_off: bool, asked: Option<u64>) -> bool {
        if !ready || put_off {
            *self = Self::default();
            return false;
        }
        if !self.standing {
            match (asked, self.held_at) {
                (None, _) => self.standing = true,
                (Some(question), None) => self.held_at = Some(question),
                (Some(question), Some(held)) if question != held => self.standing = true,
                (Some(_), Some(_)) => {}
            }
        }
        self.standing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_760_000_000_000;

    fn written() -> ResumeState {
        ResumeState {
            shape: SHAPE,
            nonce: nonce([7; 16]),
            written_at_ms: NOW - 5_000,
            gateway: Some("https://baylee.example".into()),
            account: Some(Account {
                username: Some("ada".into()),
                guest: false,
            }),
            front: Front {
                place: Place::Builder,
                decks_tab: true,
                settings: Some(section_number(Section::Updates)),
                editing: Some(Some("0199c3c2-deck".into())),
                room: None,
                offline: false,
            },
            game: Some(Game::Local {
                record: crate::bugreport::record_file_name(NOW - 60_000, 42),
                seat: 0,
                names: vec!["You".into(), "Steady 1".into()],
                hash: "00000000deadbeef".into(),
            }),
            table: Some(TableLook {
                arrangement: Some(Arrangement::Spotlight),
                visiting: Some(1),
                hand_drawer_open: true,
                sheet_folded: true,
                browser_open: false,
            }),
        }
    }

    fn handed(state: &ResumeState) -> Handoff {
        Handoff {
            nonce: state.nonce.clone(),
            session: Some("session-secret".into()),
        }
    }

    /// Everything written comes back, and the file holds nothing to sign in
    /// with: the session is only ever in the handoff.
    #[test]
    fn the_resume_state_round_trips_and_holds_no_secret() {
        let state = written();
        let text = serde_json::to_string_pretty(&state).expect("serializes");
        assert!(!text.contains("session-secret"));
        assert!(!text.contains("token"), "{text}");
        let back = accept(&text, Some(&handed(&state)), NOW).expect("taken");
        assert_eq!(back, state);
        assert_eq!(
            section_of(back.front.settings.unwrap()),
            Some(Section::Updates)
        );

        let pipe = handed(&state).to_bytes();
        assert_eq!(Handoff::from_bytes(&pipe), Some(handed(&state)));
        assert!(!format!("{:?}", handed(&state)).contains("session-secret"));
    }

    /// A file without its handoff, with another's, too old, from the
    /// future, of another shape or naming a file that is not a record is no
    /// resume. Each case is the good one with one thing changed.
    #[test]
    fn a_stale_or_foreign_file_is_not_taken() {
        let state = written();
        let text = serde_json::to_string(&state).unwrap();
        let ok = handed(&state);
        assert!(accept(&text, Some(&ok), NOW).is_ok());

        assert_eq!(accept(&text, None, NOW), Err(Refused::Foreign));
        let other = Handoff {
            nonce: nonce([8; 16]),
            ..ok.clone()
        };
        assert_eq!(accept(&text, Some(&other), NOW), Err(Refused::Foreign));
        let empty = Handoff {
            nonce: String::new(),
            ..ok.clone()
        };
        let mut blank = state.clone();
        blank.nonce = String::new();
        assert_eq!(
            accept(&serde_json::to_string(&blank).unwrap(), Some(&empty), NOW),
            Err(Refused::Foreign)
        );

        let late = NOW + FRESH_FOR_MS;
        assert_eq!(accept(&text, Some(&ok), late), Err(Refused::Stale));
        assert!(
            accept(&text, Some(&ok), late - 6_000).is_ok(),
            "just inside"
        );
        assert_eq!(
            accept(&text, Some(&ok), NOW - 120_000),
            Err(Refused::Stale),
            "written after now"
        );

        let mut shape = state.clone();
        shape.shape = SHAPE + 1;
        assert_eq!(
            accept(&serde_json::to_string(&shape).unwrap(), Some(&ok), NOW),
            Err(Refused::Unreadable)
        );
        assert_eq!(accept("{", Some(&ok), NOW), Err(Refused::Unreadable));

        let mut path = state;
        path.game = Some(Game::Local {
            record: "../../.ssh/id_ed25519".into(),
            seat: 0,
            names: vec![],
            hash: String::new(),
        });
        assert_eq!(
            accept(&serde_json::to_string(&path).unwrap(), Some(&ok), NOW),
            Err(Refused::Malformed)
        );
    }

    /// The offer never appears during my question: it waits for my answer
    /// and appears with what comes next, or at once while I am asked
    /// nothing; then it stays until I put it off.
    #[test]
    fn the_restart_offer_never_appears_during_my_question() {
        let mut offer = RestartOffer::default();
        assert!(
            !offer.step(true, false, Some(5)),
            "ready while I decide: held"
        );
        assert!(!offer.step(true, false, Some(5)), "still the same question");
        assert!(offer.step(true, false, Some(6)), "answered: it appears now");
        assert!(
            offer.step(true, false, Some(7)),
            "and stays, a panel and no question"
        );
        assert!(!offer.step(true, true, Some(7)), "put off for the session");

        let mut offer = RestartOffer::default();
        assert!(offer.step(true, false, None), "nothing asked: at once");
        assert!(
            !offer.step(false, false, None),
            "nothing ready, nothing shown"
        );
        assert!(
            !offer.step(true, false, Some(9)),
            "ready again during a question"
        );
    }

    #[test]
    fn a_nonce_is_its_bytes_in_hex() {
        assert_eq!(nonce([0xab; 16]), "ab".repeat(16));
        assert_eq!(section_of(200), None);
    }
}
