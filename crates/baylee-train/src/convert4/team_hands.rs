//! Whether the seats of a recorded game saw their teammates' hands.
//!
//! A seat's hand-sharing setting is not in the record: the table applies it
//! outside the engine (`Session::apply_team_hand_sharing`, a seat's
//! `ShowHandTo`), so the record cannot say what each seat saw. The producer of
//! a run can: `eval4` shares every team's hands on the net's tables and says
//! so in its `run.json` ([`TeamHands::RUN_KEY`]). A replay shares hands only
//! when the producer says it did, never by default: a team game of people, of
//! language models or with a shadow seat would otherwise give a seat's
//! features teammate hands it never saw.

use baylee_core::preset::GamePreset;
use baylee_gamehost::Session;
use serde_json::Value;

/// What the producer of a record says about teammates' hands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeamHands {
    /// The producer applied `Session::apply_team_hand_sharing` at the start
    /// of every game, as `eval4` does on the net's tables.
    Shared,
    /// The producer shared no hand.
    Private,
    /// The producer does not say. A game without teammates has no hand to
    /// share and replays as it is; a team game is refused.
    Unknown,
}

impl TeamHands {
    /// The `run.json` key a producer states it under: `true` when it shared,
    /// `false` when it did not.
    pub const RUN_KEY: &'static str = "team_hand_sharing";

    /// What `run` says.
    #[must_use]
    pub fn from_run(run: &Value) -> Self {
        match run.get(Self::RUN_KEY).and_then(Value::as_bool) {
            Some(true) => Self::Shared,
            Some(false) => Self::Private,
            None => Self::Unknown,
        }
    }

    /// Gives a replay `session` of `preset` the hands its producer showed.
    ///
    /// # Errors
    /// A team game whose producer does not say.
    pub(super) fn apply(self, session: &mut Session, preset: &GamePreset) -> Result<(), String> {
        match self {
            Self::Shared => session.apply_team_hand_sharing(),
            Self::Unknown if has_teammates(preset) => {
                return Err(format!(
                    "a team game whose producer does not say whether teammates' hands were \
                     shared (run.json `{}`); its seats' views cannot be rebuilt",
                    Self::RUN_KEY
                ));
            }
            // Nothing shared, or nobody to share with.
            Self::Private | Self::Unknown => {}
        }
        Ok(())
    }
}

/// Whether two seats of `preset` play on one team.
fn has_teammates(preset: &GamePreset) -> bool {
    let mut teams: Vec<u8> = preset.seats.iter().filter_map(|seat| seat.team).collect();
    teams.sort_unstable();
    teams.windows(2).any(|pair| pair[0] == pair[1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_run_says_shared_private_or_nothing() {
        assert_eq!(
            TeamHands::from_run(&json!({"team_hand_sharing": true})),
            TeamHands::Shared
        );
        assert_eq!(
            TeamHands::from_run(&json!({"team_hand_sharing": false})),
            TeamHands::Private
        );
        assert_eq!(TeamHands::from_run(&json!({})), TeamHands::Unknown);
        assert_eq!(
            TeamHands::from_run(&json!({"team_hand_sharing": "yes"})),
            TeamHands::Unknown
        );
    }
}
