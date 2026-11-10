//! `cards/lands/slow/shattered_sanctum.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Shattered Sanctum` enters tapped unless you control two or more other lands under `Coverage::Implemented`.
/// With only one other land in play, the first copy enters tapped without counting itself.
/// On the following turn, with two other lands in play, the second copy enters untapped.
#[test]
fn shattered_sanctum_counts_other_lands_and_never_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(2327, forest())
        .battlefield(0, &[forest()])
        .battlefield(1, &[forest(), forest(), forest()])
        .hand(0, &[shattered_sanctum(), shattered_sanctum()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let first = play_land(&mut engine, p0, shattered_sanctum());
    assert!(
        entered_tapped(&engine, first),
        "one Forest and this land are not two other lands"
    );

    pass_until(&mut engine, |e| e.state().turn.active == p1);
    reach_their_main_phase(&mut engine, p0);

    let second = play_land(&mut engine, p0, shattered_sanctum());
    assert!(
        !entered_tapped(&engine, second),
        "the Forest and the first land are two other lands"
    );
}
