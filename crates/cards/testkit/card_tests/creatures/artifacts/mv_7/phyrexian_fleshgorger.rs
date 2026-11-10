//! `cards/creatures/artifacts/mv_7/phyrexian_fleshgorger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Phyrexian Fleshgorger is `{7}` for a 7/5 artifact creature with menace and
/// lifelink; prototype remains unsupported (`Coverage::Partial`).
/// Life-cost ward is exercised separately in `ward_tests`.
/// What *is* written is played here in full: seven Forests pay the printed
/// cost, the Wurm resolves onto the battlefield, and the reading is taken off
/// the layer projection rather than off the card file — an artifact *and* a
/// creature, a 7/5 body, and both keywords, none of which anything else on
/// this board could have made.
#[test]
fn phyrexian_fleshgorger_resolves_as_the_seven_five_that_carries_menace_and_lifelink() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 7])
        .hand(0, &[phyrexian_fleshgorger()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // An offer is read off the *pool*, so the seven Forests are tapped
    // first: `{7}` is not castable out of untapped lands.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat holding the card holds priority");
    let card = in_hand(&engine, p0, phyrexian_fleshgorger()).expect("the Wurm is in hand");
    assert!(
        legal.castable.contains(&card),
        "seven Forests, seven mana, so `{{7}}` is payable: {:?}",
        legal.castable
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{7} is paid");
    pass_until(&mut engine, stack_is_empty);

    let wurm = on_battlefield(&engine, p0, phyrexian_fleshgorger())
        .expect("the Wurm resolved under the seat that cast it");
    let chars = engine
        .state()
        .object(wurm)
        .expect("the Wurm is an object")
        .characteristics();
    assert!(
        chars.types.contains(TypeSet::ARTIFACT) && chars.types.contains(TypeSet::CREATURE),
        "an artifact creature, not one or the other: {:?}",
        chars.types
    );
    assert_eq!(pt(&engine, wurm), (7, 5), "the body the card prints");
    let granted = keywords(&engine, wurm);
    assert!(granted.contains(KeywordSet::MENACE), "menace");
    assert!(granted.contains(KeywordSet::LIFELINK), "lifelink");
}
