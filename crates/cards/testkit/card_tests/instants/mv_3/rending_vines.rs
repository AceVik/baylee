//! `cards/instants/mv_3/rending_vines.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "64ab6dfe-ed87-47ec-afd5-a7f2e7e6bbd2"
fn rending_vines() -> CardIndex {
    card_index("64ab6dfe-ed87-47ec-afd5-a7f2e7e6bbd2")
}

/// Rending Vines ({1}{G}{G}): destroys target artifact or enchantment ONLY if
/// its mana value <= your hand size at resolution, and ALWAYS draws a card.
#[test]
fn rending_vines_destroys_only_when_mv_at_most_hand_size_and_always_draws() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[sol_ring(), index::GLORIOUS_ANTHEM, index::GRIZZLY_BEARS],
        &[rending_vines(), rending_vines()],
        &[],
    );
    let ring = lk::obj(&engine, P1, sol_ring());
    let anthem = lk::obj(&engine, P1, index::GLORIOUS_ANTHEM);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    // 1. Cast 1st Rending Vines: 1 card left in hand on resolution.
    //    Target Glorious Anthem (MV 3 > 1) -> NOT destroyed, STILL draws 1 card!
    lk::float(&mut engine, P0, &[(ManaColor::Green, 6)]);
    cast_with_floating(&mut engine, P0, rending_vines());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ring) && options.contains(&anthem));
    assert!(!options.contains(&bears), "creature is not a legal target");
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![anthem],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(
        on_battlefield(&engine, P1, index::GLORIOUS_ANTHEM).is_some(),
        "MV 3 > hand size 1 -> Anthem survives"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        2,
        "drew 1 card (1 other Rending Vines + 1 drawn)"
    );

    // 2. Cast 2nd Rending Vines: 1 card left in hand on resolution.
    //    Target Sol Ring (MV 1 <= 1) -> DESTROYED + draws 1 card!
    cast_with_floating(&mut engine, P0, rending_vines());
    lk::aim(&mut engine, &[ring], &[]);
    lk::settle(&mut engine);

    assert!(
        in_graveyard(&engine, P1, sol_ring()).is_some(),
        "MV 1 <= hand size 1 -> Sol Ring destroyed"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        2,
        "drew 1 card after casting"
    );
}
