//! `cards/instants/mv_4/ensnare.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Ensnare ({3}{U} Instant):
/// "Tap all creatures."
#[test]
fn ensnare_taps_all_creatures_on_the_battlefield() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), grizzly_bears()],
        )
        .hand(0, &[index::ENSNARE])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, P0, index::ENSNARE);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, bears));
    assert!(is_tapped(&engine, giant));
}
