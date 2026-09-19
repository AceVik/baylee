//! Which clock a table plays at.
//!
//! Every game this gateway hosted used to run the same one — 600 s to decide
//! and 60 s to reconnect — because nothing between a room and a
//! `GamePreset` ever set `HouseRules`, and the only way to play at another
//! pace was to edit a preset and rebuild. A room that cannot choose its clock
//! has exactly one pace, and every table plays at it.
//!
//! The wire needed nothing: the gateway sends the whole `GamePreset` to the
//! engine as JSON, `HouseRules` included, and gamehost has always decoded it.
//! What was missing was only a way to say which one.

use baylee_core::preset::HouseRules;

/// A named clock and the two numbers it stands for.
pub struct Preset {
    /// What a caller writes in `clock`.
    pub name: &'static str,
    /// Seconds to answer one question. Zero means no decision clock at all.
    pub decision_timeout_secs: u32,
    /// Seconds a seat may be gone before the house answers for it.
    pub reconnect_window_secs: u32,
    /// One line, for a client building a picker without hard-coding this
    /// table.
    pub blurb: &'static str,
}

/// The clocks a room may pick by name.
///
/// Four, because they are four different games rather than four points on a
/// slider. `casual` is first and is what every table played at before this
/// existed, so a caller that says nothing keeps exactly the game it had.
pub const PRESETS: &[Preset] = &[
    Preset {
        name: "casual",
        decision_timeout_secs: 600,
        reconnect_window_secs: 60,
        blurb: "ten minutes a decision; the pace every table used to play at",
    },
    Preset {
        name: "standard",
        decision_timeout_secs: 120,
        reconnect_window_secs: 60,
        blurb: "two minutes a decision",
    },
    Preset {
        name: "blitz",
        decision_timeout_secs: 30,
        reconnect_window_secs: 30,
        blurb: "thirty seconds a decision",
    },
    Preset {
        // Not the same as a very large number: the engine reads zero as "no
        // deadline" and puts nobody on a decision clock at all, so this is
        // the only way to play a game that cannot be lost on time. The
        // reconnect window stays, because it answers a different question —
        // see the refusal of zero below.
        name: "untimed",
        decision_timeout_secs: 0,
        reconnect_window_secs: 60,
        blurb: "no decision clock; a seat can still be stood in for",
    },
];

/// The shortest decision worth offering, in seconds.
///
/// Zero is allowed and means *no clock* (the engine's own reading), but one
/// through nine are not: they are not a fast game, they are a game nobody can
/// read a board in, and a room offering them is a room that wastes an
/// opponent's evening.
pub const MIN_DECISION_SECS: u32 = 10;

/// An hour to answer one question, which is already past any real table.
pub const MAX_SECS: u32 = 3600;

/// The shortest reconnect window this gateway will host.
///
/// **Zero is refused here although the engine accepts it**, and that is a
/// gateway policy rather than a rules one. A zero window means no stand-in
/// clock, so a seat whose player closed their laptop is on no clock at all
/// and the whole table waits on them forever — which is the exact failure
/// `HouseRules::reconnect_window_secs` was added to end. A local harness may
/// still choose it; a room full of strangers may not.
///
/// The argument that survives somebody making the case for private tables is
/// the second one: **nothing is lost by refusing it.** Every legitimate want
/// zero expresses — a table among friends that would rather wait than let the
/// house play — says the same thing as a very large number. Zero
/// *additionally* expresses the one thing nobody wants, and expresses it in
/// the spelling that looks most like a sensible default. A value with a
/// harmless spelling and a harmful one is refused in the harmful spelling.
///
/// [`Preset::decision_timeout_secs`] stays legal at zero for the mirror
/// reason, and the asymmetry is not an inconsistency: there is no large
/// number that means "cannot be lost on time", so there zero carries a
/// meaning nothing else can say.
pub const MIN_RECONNECT_SECS: u32 = 10;

/// Finds a named clock.
#[must_use]
pub fn named(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.name == name)
}

/// What a room asked for, resolved against the presets and the bounds.
///
/// A name picks a row; the two numbers then override whatever it gave, so
/// "blitz but I want longer to come back" needs no fifth preset and no
/// `custom` sentinel. Nothing named and nothing given is [`PRESETS`]`[0]`,
/// which is what every table already played at.
///
/// # Errors
/// A name no preset carries, or a number outside the bounds above. The
/// message names the field and what was allowed, because this reaches a
/// person building a room.
pub fn resolve(
    name: Option<&str>,
    decision: Option<u32>,
    reconnect: Option<u32>,
) -> Result<HouseRules, String> {
    let base = match name.map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => named(n).ok_or_else(|| {
            let known: Vec<&str> = PRESETS.iter().map(|p| p.name).collect();
            format!(
                "no clock called {n:?}; the ones there are: {}",
                known.join(", ")
            )
        })?,
        None => &PRESETS[0],
    };
    let decision = decision.unwrap_or(base.decision_timeout_secs);
    let reconnect = reconnect.unwrap_or(base.reconnect_window_secs);

    if decision != 0 && !(MIN_DECISION_SECS..=MAX_SECS).contains(&decision) {
        return Err(format!(
            "decision_timeout_secs must be 0 (no clock) or between \
             {MIN_DECISION_SECS} and {MAX_SECS}; got {decision}"
        ));
    }
    if !(MIN_RECONNECT_SECS..=MAX_SECS).contains(&reconnect) {
        return Err(format!(
            "reconnect_window_secs must be between {MIN_RECONNECT_SECS} and \
             {MAX_SECS}; got {reconnect}. Zero would leave a table waiting \
             forever on a player who closed their laptop"
        ));
    }

    Ok(HouseRules {
        decision_timeout_secs: decision,
        reconnect_window_secs: reconnect,
        ..HouseRules::default()
    })
}

#[cfg(test)]
mod tests {
    use super::{MAX_SECS, MIN_DECISION_SECS, MIN_RECONNECT_SECS, PRESETS, resolve};

    #[test]
    fn saying_nothing_is_the_clock_every_table_already_played_at() {
        let rules = resolve(None, None, None).expect("the default resolves");
        assert_eq!(rules.decision_timeout_secs, 600);
        assert_eq!(rules.reconnect_window_secs, 60);
    }

    #[test]
    fn a_name_picks_its_row_and_a_number_overrides_it() {
        let blitz = resolve(Some("blitz"), None, None).expect("blitz");
        assert_eq!(blitz.decision_timeout_secs, 30);
        assert_eq!(blitz.reconnect_window_secs, 30);

        // The whole reason there is no `custom` sentinel.
        let patient = resolve(Some("blitz"), None, Some(120)).expect("blitz, longer window");
        assert_eq!(patient.decision_timeout_secs, 30);
        assert_eq!(patient.reconnect_window_secs, 120);
    }

    #[test]
    fn zero_means_no_decision_clock_and_is_not_the_same_as_the_minimum() {
        let untimed = resolve(Some("untimed"), None, None).expect("untimed");
        assert_eq!(
            untimed.decision_timeout_secs, 0,
            "the engine reads zero as no deadline; a large number is a different game"
        );
        assert!(resolve(None, Some(0), None).is_ok(), "zero is a choice");
        assert!(
            resolve(None, Some(MIN_DECISION_SECS - 1), None).is_err(),
            "one second under the floor is not a fast game, it is an unreadable one"
        );
    }

    #[test]
    fn a_table_of_strangers_may_not_turn_the_stand_in_off() {
        let refused = resolve(None, None, Some(0)).expect_err("zero window is refused");
        assert!(
            refused.contains("reconnect_window_secs"),
            "the message has to name the field: {refused}"
        );
        assert!(resolve(None, None, Some(MIN_RECONNECT_SECS)).is_ok());
    }

    #[test]
    fn an_unknown_name_says_which_ones_there_are() {
        let refused = resolve(Some("bullet"), None, None).expect_err("no such clock");
        for preset in PRESETS {
            assert!(
                refused.contains(preset.name),
                "{:?} is missing from {refused}",
                preset.name
            );
        }
    }

    #[test]
    fn nothing_may_run_past_an_hour() {
        assert!(resolve(None, Some(MAX_SECS), None).is_ok());
        assert!(resolve(None, Some(MAX_SECS + 1), None).is_err());
        assert!(resolve(None, None, Some(MAX_SECS + 1)).is_err());
    }

    #[test]
    fn every_preset_satisfies_the_bounds_it_is_offered_under() {
        // A table that a caller could not have typed by hand is a table the
        // validation and the menu disagree about.
        for preset in PRESETS {
            let resolved = resolve(Some(preset.name), None, None)
                .unwrap_or_else(|e| panic!("preset {:?} does not validate: {e}", preset.name));
            assert_eq!(resolved.decision_timeout_secs, preset.decision_timeout_secs);
            assert_eq!(resolved.reconnect_window_secs, preset.reconnect_window_secs);
        }
    }
}
