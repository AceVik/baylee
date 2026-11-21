//! `cards/creatures/mv_6/wayfaring_giant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn wayfaring_giant() -> CardIndex {
    card_index("e20e5ad9-36b5-4962-acc2-555d069c5daf")
}

/// Wayfaring Giant (e20e5ad9-36b5-4962-acc2-555d069c5daf) — `{5}{W}` 1/3 Creature — Giant:
/// "Domain — This creature gets +1/+1 for each basic land type among lands you control."
#[test]
fn wayfaring_giant_gets_plus_one_plus_one_per_distinct_basic_land_type_you_control() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .battlefield(1, &[island(), swamp(), mountain(), forest()])
        .hand(0, &[wayfaring_giant(), badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, wayfaring_giant());
    pass_until(&mut engine, stack_is_empty);

    // 6 Plains = 1 distinct basic land type controlled by P0 -> 1/3 + 1/1 = 2/4.
    let giant = on_battlefield(&engine, P0, wayfaring_giant()).expect("Wayfaring Giant entered");
    assert_eq!(
        pt(&engine, giant),
        (2, 4),
        "duplicate Plains and opponent's basic land types do not count"
    );

    // Playing Badlands (Mountain Swamp) adds 2 basic land types -> 3 total -> 4/6.
    let bl = in_hand(&engine, P0, badlands()).unwrap();
    engine
        .apply(P0, PlayerAction::PlayLand { card: bl })
        .unwrap();
    assert_eq!(pt(&engine, giant), (4, 6));
}
