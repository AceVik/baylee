//! How high a creature with flying stands above the lane it is played in.
//!
//! Flying is the one evergreen keyword whose whole meaning is a *position*:
//! a creature with it cannot be blocked by a creature without it or reach
//! (CR 509.1b), and every player who has ever played the game reads that as
//! "it is up there and you are down here". The client already draws the
//! keyword as a letter on the card's rail, which says the word and not the
//! fact — so this module says the fact, and says it the way a table says it:
//! the card is off the felt and its shadow is not.
//!
//! # Why the shadow is the cue and the rise is not
//!
//! The camera leans about twenty degrees off vertical (`CAMERA_LEAN` in
//! `table.rs`), so raising a card by `h` moves its screen footprint by
//! `h * 0.36` table units and nothing else. At a duel that is about sixteen
//! screen pixels per unit of rise, and on an eight-seat ring about ten — so
//! one pixel of daylight between a card and its shadow costs a sixteenth of
//! a card's width, and any rise a player would tolerate buys two or three
//! pixels. Parallax cannot carry this on its own.
//!
//! What the eye reads instead is the *shadow*: it stays on the felt while
//! the card climbs, and it **spreads** as it goes, by the same `1 + h`
//! the table already grows a thick pile's shadow by. That is a change of
//! several pixels in the shadow's own width, it runs against the card's
//! movement rather than with it, and it is the one depth cue this camera
//! affords. The renderer draws it; the height below is chosen for it.
//!
//! # Why it moves at all
//!
//! A card held at a fixed height reads as a card someone forgot to put down.
//! The movement is the same idiom the active seat's rim uses — two periods
//! with no common multiple worth noticing, so the pose never repeats — at a
//! hover's tempo rather than a breath's, and every card on its own
//! [`phase`], so a board of four fliers is four creatures in the air and not
//! one rigid formation.
//!
//! # What this module is not allowed to do
//!
//! Nothing here may be large enough to move a card out from under a resting
//! pointer. `table.rs` has the arithmetic and the const assertions for that
//! (`covered_lift`); what matters on this side is that [`SWAY`] is small and
//! that the renderer freezes a card at [`RESTING`] while the pointer is on
//! it. A bob that toggled its own hover would be the `Out` → clear → `Over`
//! flicker the whole bound exists to prevent, and it could not be fixed by
//! growing the card, because a bob is not caused by the hover in the first
//! place.

use baylee_core::ids::ObjectId;

/// A full turn, in radians.
const TAU: f32 = std::f32::consts::TAU;

/// How high a flier rests above its lane, in table units — a card's width
/// being 1.0.
///
/// Exactly the height of a thirty-card pile, which is not a coincidence:
/// that is the tallest thing the felt ever carries and the shadow spread it
/// casts is the one this table is already calibrated on, so a flier borrows
/// a reading a player has seen before instead of inventing one. At the duel
/// camera it puts about three pixels of daylight between the card and its
/// shadow and moves each edge of that shadow six pixels outwards.
///
/// It is deliberately past `SELECTED_LIFT`, the height a card rises to when
/// it has been chosen for the pending question — see the assertions in
/// `table.rs`, where the two are compared. They still cannot be confused,
/// because every lift the pointer causes also *grows* the card and this one
/// does not: a flier is up there at its printed size, and a chosen card is
/// nearer the reader. What the renderer must not do is let one replace the
/// other: hover and selection **add** to this, or hovering a flier would
/// pull it down.
pub const RESTING: f32 = 0.18;

/// How far the bob carries it either side of [`RESTING`].
///
/// Under a quarter of the rise, and bounded from below as well as from
/// above: the trough has to stay clear of `SELECTED_LIFT`, so that a card
/// the player has chosen is never standing higher than a creature that
/// flies. On the card itself this is about half a pixel of travel — the bob
/// is read in the shadow's width, not in the card's position, which is also
/// why it is safe to leave under a pointer.
pub const SWAY: f32 = 0.04;
// The proportion is what makes it a float rather than a bounce, so it is held
// here and not left to whoever edits one of the two numbers next.
const _: () = assert!(SWAY * 4.0 < RESTING);

/// The slower of the two periods, in seconds: a long rise and fall.
pub const DRIFT_SECONDS: f32 = 3.7;

/// The quicker of the two, in seconds.
///
/// 3.7 and 2.3 are thirty-seven and twenty-three tenths of a second, both
/// prime, so the pair comes back into phase once every 85 seconds — longer
/// than anybody watches one creature, which is the whole point of choosing
/// two periods instead of one. The seat rim makes the same argument with 11
/// and 7; a flier is quicker than a breath, because it is holding itself up
/// rather than resting.
pub const SWAY_SECONDS: f32 = 2.3;

/// How much of the travel the slow period carries.
///
/// More than half, because it is the one that reads as floating. An even
/// split lets the quick period dominate the *look* of the motion — the
/// derivative, not the amplitude, is what an eye calls speed — and the card
/// starts to vibrate instead of hang.
const DRIFT_SHARE: f32 = 0.625;

/// Where in the cycle this particular card is.
///
/// A number in `[0, 1)`, spread with the golden ratio in fixed point: the
/// additive recurrence `slot * ⌊2³²/φ⌋` is the one-dimensional sequence that
/// keeps every prefix as evenly spaced as a prefix can be, so two creatures
/// cast one after the other — which is what consecutive slots usually are —
/// land as far apart in the cycle as anything ever does. No `rand` and no
/// clock: two players watching the same board see the same creature at the
/// same point of its bob.
#[must_use]
pub fn phase(object: ObjectId) -> f32 {
    /// 2³² divided by the golden ratio, rounded to odd so the recurrence
    /// visits every residue.
    const GOLDEN: u32 = 2_654_435_769;
    #[expect(
        clippy::cast_precision_loss,
        reason = "shifted to 24 bits first, which a f32 mantissa holds exactly"
    )]
    let unit = (object.slot().wrapping_mul(GOLDEN) >> 8) as f32;
    unit / 16_777_216.0
}

/// How high a flier stands `elapsed` seconds into the game.
///
/// `elapsed` is the application's own clock and `phase` this card's offset
/// in it, so the two sines are one waveform sampled at a different place per
/// card rather than two independently-drifting cards. The result is always
/// within [`SWAY`] of [`RESTING`], because the two shares sum to one.
#[must_use]
pub fn height(elapsed: f32, phase: f32) -> f32 {
    let t = elapsed + phase * DRIFT_SECONDS;
    let drift = (TAU * t / DRIFT_SECONDS).sin();
    let sway = (TAU * t / SWAY_SECONDS).sin();
    RESTING + SWAY * (DRIFT_SHARE * drift + (1.0 - DRIFT_SHARE) * sway)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    /// Every tenth of a second of a full beat of the two periods.
    fn over_a_cycle() -> impl Iterator<Item = f32> {
        (0..1500).map(|i| i as f32 / 10.0)
    }

    #[test]
    fn a_flier_never_comes_back_down_to_the_felt() {
        let low = over_a_cycle()
            .map(|t| height(t, phase(obj(7))))
            .fold(f32::INFINITY, f32::min);
        assert!(
            low >= RESTING - SWAY - 1e-5,
            "the bob left the band it is allowed: {low} against {}",
            RESTING - SWAY
        );
        assert!(low > 0.0, "a flier standing on the table is not flying");
    }

    #[test]
    fn the_bob_never_climbs_past_the_band_it_is_given() {
        let high = over_a_cycle()
            .map(|t| height(t, phase(obj(7))))
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            high <= RESTING + SWAY + 1e-5,
            "the bob left the band it is allowed: {high}"
        );
    }

    #[test]
    fn a_board_of_fliers_does_not_pulse_as_one_body() {
        let together: Vec<f32> = (1..=6).map(|slot| height(0.0, phase(obj(slot)))).collect();
        for (i, a) in together.iter().enumerate() {
            for b in &together[i + 1..] {
                assert!(
                    (a - b).abs() > 1e-3,
                    "two creatures cast one after the other are at the same point of the bob: {together:?}"
                );
            }
        }
    }

    #[test]
    fn the_two_periods_never_bring_the_pose_back() {
        // One slow period later the quick one has turned 4.7/3.1 times —
        // half a turn out — so the card is somewhere else. This is the
        // property a single sine would fail, and it is what makes the
        // movement read as alive rather than as a loop.
        let apart = over_a_cycle()
            .map(|t| (height(t, 0.0) - height(t + DRIFT_SECONDS, 0.0)).abs())
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            apart > SWAY * 0.5,
            "a slow period later the card is back where it was: {apart}"
        );
    }

    #[test]
    fn the_same_card_is_in_the_same_place_for_everyone() {
        // No clock and no `rand` in the phase: a replay, a spectator and the
        // player whose creature it is all see one bob.
        assert!((phase(obj(41)) - phase(obj(41))).abs() < f32::EPSILON);
        assert!(
            (0..64).all(|slot| (0.0..1.0).contains(&phase(obj(slot)))),
            "a phase outside one turn of the cycle"
        );
    }
}
