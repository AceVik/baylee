//! `cards/lands/utility/oboro_palace_in_the_clouds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Oboro, Palace in the Clouds: "{T}: Add {U}." and "{1}: Return Oboro to its owner's hand."
/// Oboro taps for blue mana, which remains in the pool to pay for its own return ability.
/// The ability resolves, returning the tapped land back to its owner's hand.
// Was red: the ability resolved and Oboro did not move, because
// `Effect::ReturnToHand` threw its own `TargetSpec` away and read the targets
// chosen at activation — of which there are none, since naming your own
// source chooses nothing. `resolve::zones::spec_object` reads the spec; the
// rule's own tests are in `engine::this_object_tests` (#147).
#[test]
fn oboro_palace_in_the_clouds_taps_for_blue_and_returns_itself_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(14, forest())
        .battlefield(0, &[oboro_palace_in_the_clouds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let oboro = on_battlefield(&engine, p0, oboro_palace_in_the_clouds()).expect("Oboro deployed");
    activate(&mut engine, p0, oboro_palace_in_the_clouds(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "Oboro tapped for {{U}}"
    );
    assert!(is_tapped(&engine, oboro));

    activate(&mut engine, p0, oboro_palace_in_the_clouds(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, oboro_palace_in_the_clouds()).is_none(),
        "Oboro left the battlefield"
    );
    assert!(
        in_hand(&engine, p0, oboro_palace_in_the_clouds()).is_some(),
        "Oboro returned to owner's hand"
    );
}
