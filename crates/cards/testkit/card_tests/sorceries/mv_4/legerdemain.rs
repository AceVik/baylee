//! `cards/sorceries/mv_4/legerdemain.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn legerdemain() -> CardIndex {
    card_index("107d5f67-1395-444f-b612-c5cc770df956")
}

/// Legerdemain (`107d5f67-1395-444f-b612-c5cc770df956`) — {2}{U}{U} Sorcery:
/// "Exchange control of target artifact or creature and another target permanent that shares one of those types with it."
#[test]
fn legerdemain_exchanges_control_of_two_creatures() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[island(), island(), island(), island(), index::GRIZZLY_BEARS],
        )
        .battlefield(1, &[index::HILL_GIANT, index::JAYEMDAE_TOME])
        .hand(0, &[legerdemain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, P0, legerdemain());
    let tome = on_battlefield(&engine, P1, index::JAYEMDAE_TOME).unwrap();
    let land = on_battlefield(&engine, P0, island()).unwrap();
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![land],
                    players: vec![]
                }
            )
            .is_err(),
        "first target must be artifact or creature"
    );
    aim_at(&mut engine, P0, bears);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears],
                    players: vec![]
                }
            )
            .is_err(),
        "another target permanent"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![tome],
                    players: vec![]
                }
            )
            .is_err(),
        "a plain artifact shares no printed type with a creature"
    );
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
