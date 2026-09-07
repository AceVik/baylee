//! The second life total, as a track (CR 903.10a).
//!
//! Twenty-one combat damage from a single commander kills a player whatever
//! their life total says, so a seat that has been hit by commanders is on two
//! clocks at once and only one of them is the number in the panel. This module
//! turns the raw tally into what a bar needs, and nothing here draws: the same
//! split as [`crate::cardplate`] and [`crate::cardrail`], and for the same
//! reason — the threshold arithmetic is the part worth testing, and it cannot
//! be tested through a renderer.
//!
//! # Why a track and not seven numbers
//!
//! The rule is *per commander*, so a seat facing a table of seven has seven
//! separate counts and no total that means anything: 20 from one commander is
//! lethal next hit, and 20 spread over seven is barely a scratch. Adding them
//! would invent a number Magic does not have. Listing them all would put a
//! table in a panel that has room for one line.
//!
//! So the track answers the question a player actually asks — *how close am I
//! to dying to this* — by drawing the worst source as a fill and every other
//! source as a tick on the same scale. The fill is the clock that is about to
//! run out; the ticks say how many others are behind it and how far.

use baylee_view::CommanderDamage;

/// Damage from one commander that ends the game (CR 903.10a).
pub const LETHAL: u16 = 21;

/// How close to [`LETHAL`] counts as danger.
///
/// Five, to match the life total's own rule in the same panel: a player who
/// has learned that red life means "one more spell" should not have to learn a
/// second threshold for the bar underneath it.
pub const DANGER_MARGIN: u16 = 5;

/// A seat's commander damage, ready to draw.
#[derive(Clone, PartialEq, Debug)]
pub struct Track {
    /// The largest single tally — the one that decides the game.
    pub worst: u16,
    /// That tally as a fraction of [`LETHAL`], clamped to `0.0..=1.0`.
    ///
    /// Clamped because a single hit can carry a seat past twenty-one before
    /// state-based actions run (CR 704.3), and a bar drawn past its own end is
    /// a rendering bug rather than a rules one.
    pub fill: f32,
    /// Every *other* source, as fractions of [`LETHAL`], ascending.
    ///
    /// Sources tied with the worst are not repeated here — the fill's edge is
    /// already standing at that mark, and a tick drawn on top of it would say
    /// "two commanders" with one line. Two sources tied below the worst
    /// collapse into one tick for the same reason. That loses which commander
    /// is which, which the tooltip answers; the same half-answer
    /// [`crate::board::Chip`] gives for counter colours.
    pub ticks: Vec<f32>,
    /// Whether the worst source is within [`DANGER_MARGIN`] of lethal.
    pub danger: bool,
}

impl Track {
    /// The track for a seat's tally, or `None` when no commander has hit it.
    ///
    /// `None` rather than an empty track, so the panel can leave the line out
    /// entirely — the same rule poison and energy already follow. A seat at a
    /// Commander table that has taken no commander damage is the normal case,
    /// and a bar sitting at zero under every seat all game would be noise.
    #[must_use]
    pub fn of(damage: &[CommanderDamage]) -> Option<Self> {
        let worst = damage.iter().map(|d| d.amount).max()?;
        let scale = f32::from(LETHAL);
        let mut ticks: Vec<u16> = damage
            .iter()
            .map(|d| d.amount)
            .filter(|&a| a < worst)
            .collect();
        ticks.sort_unstable();
        ticks.dedup();
        Some(Self {
            worst,
            fill: (f32::from(worst) / scale).clamp(0.0, 1.0),
            ticks: ticks
                .into_iter()
                .map(|a| (f32::from(a) / scale).clamp(0.0, 1.0))
                .collect(),
            danger: worst + DANGER_MARGIN >= LETHAL,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{DANGER_MARGIN, LETHAL, Track};
    use baylee_core::ids::ObjectId;
    use baylee_view::CommanderDamage;

    fn hit(slot: u32, amount: u16) -> CommanderDamage {
        CommanderDamage {
            source: ObjectId::new(slot, 0),
            amount,
        }
    }

    #[test]
    fn a_seat_no_commander_has_hit_has_no_track_at_all() {
        assert_eq!(Track::of(&[]), None);
    }

    /// The whole reason this is not a sum: the worst source is the clock.
    #[test]
    fn the_fill_is_the_worst_source_and_never_the_total() {
        let track = Track::of(&[hit(1, 7), hit(2, 6), hit(3, 5)]).expect("three sources");
        // Eighteen in total, which would be a nearly full bar and a lie —
        // this seat is not close to dying to any of them.
        assert_eq!(track.worst, 7);
        assert!((track.fill - 7.0 / 21.0).abs() < 1e-6);
        assert_eq!(track.ticks.len(), 2, "the other two sources");
        assert!(!track.danger);
    }

    #[test]
    fn the_other_sources_are_ticks_in_ascending_order() {
        let track = Track::of(&[hit(1, 3), hit(2, 12), hit(3, 9)]).expect("three sources");
        assert_eq!(track.worst, 12);
        let ticks: Vec<f32> = vec![3.0 / 21.0, 9.0 / 21.0];
        assert_eq!(track.ticks.len(), ticks.len());
        for (got, want) in track.ticks.iter().zip(&ticks) {
            assert!((got - want).abs() < 1e-6, "{got} != {want}");
        }
    }

    /// A tick standing exactly where the fill ends would draw one line and
    /// mean two commanders, which is the one thing the ticks are for.
    #[test]
    fn a_source_tied_with_the_worst_is_not_also_a_tick() {
        let track = Track::of(&[hit(1, 10), hit(2, 10)]).expect("two sources");
        assert_eq!(track.worst, 10);
        assert!(track.ticks.is_empty());
    }

    #[test]
    fn two_sources_on_the_same_mark_collapse_to_one_tick() {
        let track = Track::of(&[hit(1, 15), hit(2, 4), hit(3, 4)]).expect("three sources");
        assert_eq!(track.ticks.len(), 1);
    }

    /// Partners are two commanders and two thresholds, not one shared clock.
    #[test]
    fn partners_are_two_sources_and_neither_helps_the_other_kill() {
        let track = Track::of(&[hit(1, 20), hit(2, 20)]).expect("two sources");
        assert_eq!(track.worst, 20, "forty damage, and still one hit from dead");
        assert!(track.fill < 1.0);
        assert!(track.danger);
    }

    #[test]
    fn danger_starts_exactly_five_short_of_lethal() {
        let edge = LETHAL - DANGER_MARGIN;
        assert!(!Track::of(&[hit(1, edge - 1)]).expect("a source").danger);
        assert!(Track::of(&[hit(1, edge)]).expect("a source").danger);
    }

    /// State-based actions have not run yet when the view is built, so the
    /// tally really can read past twenty-one for one frame.
    #[test]
    fn an_overkill_fills_the_track_and_does_not_run_past_its_end() {
        let track = Track::of(&[hit(1, 40)]).expect("a source");
        assert_eq!(track.worst, 40);
        assert!((track.fill - 1.0).abs() < 1e-6);
        assert!(track.danger);
    }
}
