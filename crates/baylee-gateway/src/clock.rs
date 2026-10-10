//! Which clock a table plays at.
//!
//! Every game this gateway hosted used to run the same one — 600 s to decide
//! and 60 s to reconnect — because nothing between a room and a
//! `GamePreset` ever set `HouseRules`, and the only way to play at another
//! pace was to edit a preset and rebuild. A room that cannot choose its clock
//! has exactly one pace, and every table plays at it. Since 08.10.2026 a room
//! that says nothing plays `classic`, three minutes a decision (the owner).
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
    /// One line, for a client building a picker without hard-coding this
    /// table.
    pub blurb: &'static str,
}

/// The clocks a room may pick by name.
///
/// The first is the default, the clock a room that names none plays:
/// `classic`, three minutes a decision (owner, 08.10.2026), the same number
/// as [`HouseRules::default`]. `casual` was first until then, and every
/// table played at its ten minutes; it is still offered, as is every other
/// clock that was. After the default they run from the fastest to the
/// slowest, then `untimed` (owner, 10.10.2026: many more paces, from fifteen
/// seconds to an hour). A name is never taken back or given another pace:
/// rooms, rematches and older clients name them.
pub const PRESETS: &[Preset] = &[
    Preset {
        name: "classic",
        decision_timeout_secs: baylee_core::preset::DEFAULT_DECISION_SECS,
        blurb: "three minutes a decision; the default",
    },
    Preset {
        name: "bullet",
        decision_timeout_secs: 15,
        blurb: "fifteen seconds a decision",
    },
    Preset {
        name: "blitz",
        decision_timeout_secs: 30,
        blurb: "thirty seconds a decision",
    },
    Preset {
        name: "rapid",
        decision_timeout_secs: 45,
        blurb: "forty-five seconds a decision",
    },
    Preset {
        name: "quick",
        decision_timeout_secs: 60,
        blurb: "one minute a decision",
    },
    Preset {
        name: "brisk",
        decision_timeout_secs: 90,
        blurb: "ninety seconds a decision",
    },
    Preset {
        name: "standard",
        decision_timeout_secs: 120,
        blurb: "two minutes a decision",
    },
    Preset {
        name: "relaxed",
        decision_timeout_secs: 300,
        blurb: "five minutes a decision",
    },
    Preset {
        name: "casual",
        decision_timeout_secs: 600,
        blurb: "ten minutes a decision",
    },
    Preset {
        name: "leisurely",
        decision_timeout_secs: 900,
        blurb: "fifteen minutes a decision",
    },
    Preset {
        name: "patient",
        decision_timeout_secs: 1200,
        blurb: "twenty minutes a decision",
    },
    Preset {
        name: "unhurried",
        decision_timeout_secs: 1800,
        blurb: "half an hour a decision",
    },
    Preset {
        name: "marathon",
        decision_timeout_secs: 3600,
        blurb: "an hour a decision",
    },
    Preset {
        // Not the same as a very large number: the engine reads zero as "no
        // deadline" and puts nobody on a decision clock at all, so this is
        // the only way to play a game that cannot be lost on time. The
        // reconnect window stays, because it answers a different question —
        // see the refusal of zero below.
        name: "untimed",
        decision_timeout_secs: 0,
        blurb: "no decision clock; a seat can still be stood in for",
    },
];

/// How long a seat whose connection was lost is waited for before the
/// house plays it, when another player is still at the table (owner,
/// 10.10.2026: three minutes). One number for every clock rather than a
/// column of the table above: it is how long a router takes to come back,
/// not how fast the game is played. `BAYLEE_RECONNECT_SECS` replaces it for
/// a whole gateway, and a room may still name its own
/// (`reconnect_window_secs`). A seat with no other player left at the table
/// is not waited for on any clock: the game pauses
/// (`docs/protocol.md` §"Leaving, and losing the connection").
pub const DEFAULT_RECONNECT_SECS: u32 = 180;

/// `BAYLEE_RECONNECT_SECS`, read at start: unset is
/// [`DEFAULT_RECONNECT_SECS`], and a value outside the bounds a room is
/// held to refuses startup.
///
/// # Errors
/// The sentence the gateway refuses to start with.
pub fn reconnect_from_env(raw: Option<&str>) -> Result<u32, String> {
    match raw.map(str::trim) {
        None | Some("") => Ok(DEFAULT_RECONNECT_SECS),
        Some(text) => match text.parse::<u32>() {
            Ok(secs) if (MIN_RECONNECT_SECS..=MAX_SECS).contains(&secs) => Ok(secs),
            Ok(secs) => Err(format!(
                "{secs} seconds is outside {MIN_RECONNECT_SECS}..={MAX_SECS}"
            )),
            Err(_) => Err(format!("{text:?} is not a whole number of seconds")),
        },
    }
}

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
/// "blitz but I want longer to come back" needs no extra preset and no
/// `custom` sentinel. Nothing named and nothing given is [`PRESETS`]`[0]`,
/// the default.
///
/// # Errors
/// A name no preset carries, or a number outside the bounds above. The
/// message names the field and what was allowed, because this reaches a
/// person building a room.
pub fn resolve(
    name: Option<&str>,
    decision: Option<u32>,
    reconnect: Option<u32>,
    default_reconnect: u32,
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
    let reconnect = reconnect.unwrap_or(default_reconnect);

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
    use super::{
        DEFAULT_RECONNECT_SECS, MAX_SECS, MIN_DECISION_SECS, MIN_RECONNECT_SECS, PRESETS,
        reconnect_from_env, resolve,
    };

    /// Three minutes a decision for a room that names no clock (owner,
    /// 08.10.2026), and the same number a preset built without a room
    /// gets: a table opened from the lobby and one opened by `dev-table`
    /// or a rematch's defaults play at one pace.
    #[test]
    fn saying_nothing_is_three_minutes_a_decision() {
        let rules = resolve(None, None, None, 180).expect("the default resolves");
        assert_eq!(rules.decision_timeout_secs, 180);
        assert_eq!(rules.reconnect_window_secs, DEFAULT_RECONNECT_SECS);
        assert_eq!(PRESETS[0].name, "classic");
        assert_eq!(
            PRESETS[0].decision_timeout_secs,
            baylee_core::preset::HouseRules::default().decision_timeout_secs,
            "the gateway's default clock and the house rules' default disagree"
        );
        // Every clock that was offered still is, at its own pace.
        for (name, decide) in [
            ("casual", 600),
            ("standard", 120),
            ("blitz", 30),
            ("untimed", 0),
        ] {
            let rules = resolve(Some(name), None, None, 180).expect("still offered");
            assert_eq!(rules.decision_timeout_secs, decide, "{name}");
        }
    }

    /// Many paces (owner, 10.10.2026): after the default they run from
    /// fifteen seconds to an hour, no two alike, and `untimed` closes the
    /// list. A menu with two rows of one pace would make the client's
    /// `table_clock_label`, which finds a table's clock by its numbers,
    /// name a room after the wrong one.
    #[test]
    fn the_paces_run_from_fifteen_seconds_to_an_hour_each_once() {
        let names: Vec<&str> = PRESETS.iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            [
                "classic",
                "bullet",
                "blitz",
                "rapid",
                "quick",
                "brisk",
                "standard",
                "relaxed",
                "casual",
                "leisurely",
                "patient",
                "unhurried",
                "marathon",
                "untimed",
            ]
        );
        let timed: Vec<u32> = PRESETS[1..PRESETS.len() - 1]
            .iter()
            .map(|p| p.decision_timeout_secs)
            .collect();
        assert_eq!(
            timed,
            [15, 30, 45, 60, 90, 120, 300, 600, 900, 1200, 1800, 3600]
        );
        let mut paces: Vec<u32> = PRESETS.iter().map(|p| p.decision_timeout_secs).collect();
        paces.sort_unstable();
        paces.dedup();
        assert_eq!(paces.len(), PRESETS.len(), "two clocks of one pace");
        assert!(PRESETS.iter().all(|p| !p.blurb.is_empty()));
    }

    #[test]
    fn a_name_picks_its_row_and_a_number_overrides_it() {
        let blitz = resolve(Some("blitz"), None, None, 180).expect("blitz");
        assert_eq!(blitz.decision_timeout_secs, 30);
        assert_eq!(
            blitz.reconnect_window_secs, 180,
            "one window for every clock"
        );

        // The whole reason there is no `custom` sentinel.
        let patient = resolve(Some("blitz"), None, Some(120), 180).expect("blitz, longer window");
        assert_eq!(patient.decision_timeout_secs, 30);
        assert_eq!(patient.reconnect_window_secs, 120);
    }

    #[test]
    fn zero_means_no_decision_clock_and_is_not_the_same_as_the_minimum() {
        let untimed = resolve(Some("untimed"), None, None, 180).expect("untimed");
        assert_eq!(
            untimed.decision_timeout_secs, 0,
            "the engine reads zero as no deadline; a large number is a different game"
        );
        assert!(
            resolve(None, Some(0), None, 180).is_ok(),
            "zero is a choice"
        );
        assert!(
            resolve(None, Some(MIN_DECISION_SECS - 1), None, 180).is_err(),
            "one second under the floor is not a fast game, it is an unreadable one"
        );
    }

    #[test]
    fn a_table_of_strangers_may_not_turn_the_stand_in_off() {
        let refused = resolve(None, None, Some(0), 180).expect_err("zero window is refused");
        assert!(
            refused.contains("reconnect_window_secs"),
            "the message has to name the field: {refused}"
        );
        assert!(resolve(None, None, Some(MIN_RECONNECT_SECS), 180).is_ok());
    }

    #[test]
    fn an_unknown_name_says_which_ones_there_are() {
        let refused = resolve(Some("glacial"), None, None, 180).expect_err("no such clock");
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
        assert!(resolve(None, Some(MAX_SECS), None, 180).is_ok());
        assert!(resolve(None, Some(MAX_SECS + 1), None, 180).is_err());
        assert!(resolve(None, None, Some(MAX_SECS + 1), 180).is_err());
    }

    #[test]
    fn every_preset_satisfies_the_bounds_it_is_offered_under() {
        // A table that a caller could not have typed by hand is a table the
        // validation and the menu disagree about.
        for preset in PRESETS {
            let resolved = resolve(Some(preset.name), None, None, 180)
                .unwrap_or_else(|e| panic!("preset {:?} does not validate: {e}", preset.name));
            assert_eq!(resolved.decision_timeout_secs, preset.decision_timeout_secs);
        }
    }

    #[test]
    fn the_gateway_names_the_window_every_room_gets_by_default() {
        assert_eq!(reconnect_from_env(None), Ok(180));
        assert_eq!(reconnect_from_env(Some(" ")), Ok(180));
        assert_eq!(reconnect_from_env(Some("45")), Ok(45));
        assert!(reconnect_from_env(Some("0")).is_err());
        assert!(reconnect_from_env(Some("3601")).is_err());
        assert!(reconnect_from_env(Some("soon")).is_err());
        let rules = resolve(Some("blitz"), None, None, 45).expect("blitz");
        assert_eq!(rules.reconnect_window_secs, 45);
        let own = resolve(Some("blitz"), None, Some(20), 45).expect("its own");
        assert_eq!(own.reconnect_window_secs, 20, "a room's own number wins");
    }
}
