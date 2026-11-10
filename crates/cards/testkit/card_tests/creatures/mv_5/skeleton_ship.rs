//! `cards/creatures/mv_5/skeleton_ship.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn skeleton_ship() -> CardIndex {
    card_index("8c85887d-5935-46ec-a075-dc9f5133bb96")
}

/// Skeleton Ship (8c85887d-5935-46ec-a075-dc9f5133bb96) — {3}{U}{B} 0/3 Legendary Creature — Skeleton:
/// - "When you control no Islands, sacrifice Skeleton Ship."
/// - "{T}: Put a -1/-1 counter on target creature."
#[test]
fn skeleton_ship_places_minus_one_minus_one_counters_and_sacrifices_when_you_control_no_islands() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                skeleton_ship(),
                island(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[stone_rain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, bears), (2, 2));

    // 1. Activate {T}: Put a -1/-1 counter on target creature (Grizzly Bears).
    activate(&mut engine, P0, skeleton_ship(), 1);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, bears),
        (1, 1),
        "Grizzly Bears has a -1/-1 counter from Skeleton Ship"
    );

    // 2. Destroy P0's only Island with Stone Rain -> state trigger sacrifices Skeleton Ship.
    let isl = on_battlefield(&engine, P0, island()).unwrap();
    cast_from_hand(&mut engine, P0, stone_rain());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![isl],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P0, skeleton_ship()).is_none()
            && in_graveyard(&engine, P0, skeleton_ship()).is_some(),
        "Skeleton Ship is sacrificed when P0 controls no Islands"
    );
}
