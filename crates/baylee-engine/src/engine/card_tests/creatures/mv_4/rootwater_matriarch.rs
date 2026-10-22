//! `cards/creatures/mv_4/rootwater_matriarch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT, HOLY_STRENGTH};

fn rootwater_matriarch() -> CardIndex {
    card_index("1d260526-db57-486d-b0bc-d523139a0148")
}

/// `Rootwater Matriarch` ({2}{U}{U}, 2/3 Creature — Merfolk, `Coverage::Partial`):
/// `{T}: Gain control of target enchanted creature for as long as it remains enchanted.`
#[test]
fn rootwater_matriarch_gains_control_of_target_enchanted_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), rootwater_matriarch()])
        .battlefield(1, &[GRIZZLY_BEARS, HILL_GIANT])
        .hand(0, &[HOLY_STRENGTH])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    // Enchant Grizzly Bears with Holy Strength.
    cast_from_hand(&mut engine, p0, HOLY_STRENGTH);
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

    activate(&mut engine, p0, rootwater_matriarch(), 0);
    // Unenchanted Hill Giant cannot be targeted.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![giant],
                    players: vec![],
                },
            )
            .is_err()
    );

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

    assert_eq!(engine.state().object(bears).unwrap().controller, p0);
}
