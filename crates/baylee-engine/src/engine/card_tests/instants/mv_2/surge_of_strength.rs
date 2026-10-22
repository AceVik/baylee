//! `cards/instants/mv_2/surge_of_strength.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn surge_of_strength() -> CardIndex {
    card_index("bd3e55cf-ffca-402c-91ea-2420035a9dd1")
}

/// Surge of Strength ({R}{G} Instant):
/// - "As an additional cost to cast this spell, discard a red or green card."
/// - "Target creature gains trample and gets +X/+0 until end of turn, where X is that creature's mana value."
#[test]
fn surge_of_strength_discards_red_or_green_card_and_pumps_by_target_mana_value_with_trample() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[HILL_GIANT, mountain(), forest()])
        .hand(0, &[surge_of_strength(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let bears = in_hand(&engine, p0, grizzly_bears()).unwrap();
    let surge = in_hand(&engine, p0, surge_of_strength()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: surge })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    // Hill Giant ({3}{R}, mana value 4, 3/3) gets +4/+0 and trample -> 7/3 with trample.
    assert_eq!(pt(&engine, giant), (7, 3));
    assert!(keywords(&engine, giant).contains(KeywordSet::TRAMPLE));
}
