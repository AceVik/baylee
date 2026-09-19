//! What the table *draws* on a land a payment window would tap.
//!
//! Both ends of this were tested and the join was not. `Offer::on` has unit
//! tests in `cardmat`, and `Duel::owed_plan` has `owed_tests` — and between
//! them sat the one line that makes either of them visible to a player:
//! [`super::placements`] passing `duel.proposing()` into the offer it builds
//! every card with. A test per end and none across the join is how "declared
//! but never wired" ships, which is the bug `library_fan_tests` one module
//! down exists to prevent and the one `cue_feed_tests` says this client has
//! shipped before.
//!
//! It is also the answer to a question #105 asked from the other side. The
//! trap there is a `Duel` a struct literal built, and `owed_plan` is the
//! field that proves it: a literal leaves it `None`, `proposing()` then
//! answers `Nothing`, and every land is drawn dark — which is a legal answer,
//! so the test passes and says nothing. That is why the seat below comes from
//! [`crate::owed_tests::seat_with_two_forests`] rather than from a second
//! harness written here: one wire-built seat, asked two questions.
//!
//! **What these do not reach**, said out loud so the next reader does not take
//! the coverage for wider than it is. Two Forests at this table are two board
//! groups of one card each — measured, not assumed — so `group.members` is a
//! single id at every call site below and the *stack* case is untested here.
//! That case is the reason the offer is resolved in [`super::placements`]
//! rather than in the sync loop, and it is held next door in `cardmat`, over a
//! three-member slice.

use super::{Placement, placements};
use crate::owed_tests::seat_with_two_forests;
use baylee_client_core::layout::TableLayout;
use baylee_core::ids::PlayerId;
use baylee_core::mana::ManaCost;

/// The seat, drawn. A layout is a literal on purpose — it is computed from
/// the seat list and the aspect ratio and from nothing on the wire, so there
/// is no fill path for one to go through.
fn drawn(owed: Option<ManaCost>) -> Vec<Placement> {
    let mut duel = seat_with_two_forests(owed);
    duel.layout = Some(TableLayout::new(&[PlayerId::new(0)], 16.0 / 9.0, None));
    crate::rebuild_board(&mut duel);
    placements(&duel)
}

/// Cards the table would light as about to tap.
fn lit(drawn: &[Placement]) -> usize {
    drawn.iter().filter(|p| p.offer.will_tap).count()
}

/// A payment window reaches the felt: the two Forests that would settle it
/// are drawn lit.
///
/// The number is two and not "more than none" because the plan is what
/// decides it — a client that lit every land it owned would also pass a
/// non-empty assertion, and that is the failure this whole field exists to
/// prevent.
#[test]
fn the_table_lights_the_lands_a_payment_window_would_tap() {
    let drawn = drawn(Some(ManaCost::parse("{1}{G}")));
    assert!(!drawn.is_empty(), "the table drew nothing at all");
    assert_eq!(
        lit(&drawn),
        2,
        "{{1}}{{G}} is paid by both Forests, so both are drawn about to tap"
    );
}

/// And the quiet window it is shaped exactly like reaches it dark.
///
/// Without this one the test above is satisfied by a client that lights
/// lands unconditionally, which is the same shape as the defect
/// `a_quiet_priority_window_over_untapped_lands_lights_nothing` guards one
/// layer down — asked again here because a layer that is right is not
/// evidence about the layer that draws it.
#[test]
fn the_table_lights_nothing_in_the_quiet_window_it_is_shaped_like() {
    let drawn = drawn(None);
    assert!(!drawn.is_empty(), "the table drew nothing at all");
    assert_eq!(lit(&drawn), 0, "a quiet window lit a land");
}

/// The lit lands are the ones the **plan** names, and not every land the
/// seat owns.
///
/// `{G}` is paid by one Forest, so one of the two is drawn about to tap.
/// Without this, a client that lit every untapped land whenever anything was
/// owed would satisfy the first test exactly — and that client is the one
/// this feature was built not to be.
#[test]
fn the_lit_lands_are_the_ones_the_plan_names() {
    let drawn = drawn(Some(ManaCost::parse("{G}")));
    assert_eq!(
        lit(&drawn),
        1,
        "{{G}} needs one Forest, so one card is drawn about to tap"
    );
}

/// Owed is not the same as payable, and only the second lights a land.
///
/// `{2}{G}` is three mana and this seat has two Forests, so `manaplan::plan`
/// returns nothing at all — `proposing()` answers `Owed` only when there is a
/// plan to propose. That separates the two halves of the sentence: a client
/// keyed on `view.owed` alone would light both lands here and be offering to
/// tap them towards a payment they cannot complete, which is the one mistake
/// that costs a player their turn's mana rather than merely misinforming them.
#[test]
fn a_debt_this_table_cannot_settle_lights_nothing() {
    let drawn = drawn(Some(ManaCost::parse("{2}{G}")));
    assert!(!drawn.is_empty(), "the table drew nothing at all");
    assert_eq!(
        lit(&drawn),
        0,
        "a land lit towards a payment it cannot finish"
    );
}
