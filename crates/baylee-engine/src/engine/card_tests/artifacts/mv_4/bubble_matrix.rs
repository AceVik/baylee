//! `cards/artifacts/mv_4/bubble_matrix.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn bubble_matrix() -> CardIndex {
    card_index("50261dca-bfeb-40b3-95b6-9057491ef3ad")
}

/// `Bubble Matrix` ({4}, Artifact) prevents all damage that would be dealt to
/// creatures.
#[test]
fn bubble_matrix_prevents_all_damage_to_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[bubble_matrix(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, bubble_matrix());
    pass_until(&mut engine, stack_is_empty);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_some());
}

/// "Damage that would be dealt to creatures" is every creature's, whoever
/// controls it, and only creatures': an opposing Hill Giant takes nothing from
/// a Lightning Bolt, and the same Bolt at a player still deals its 3.
#[test]
fn bubble_matrix_shields_any_creature_but_not_a_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[bubble_matrix(), mountain(), mountain()])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, HILL_GIANT).is_some());
    assert_eq!(lk::damage_on(&engine, giant), 0, "the damage was prevented");

    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(lk::life(&engine, p1), 17, "a player is not a creature");
}
