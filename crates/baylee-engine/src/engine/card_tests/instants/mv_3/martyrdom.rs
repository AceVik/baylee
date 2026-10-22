//! `cards/instants/mv_3/martyrdom.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::life;

fn martyrdom() -> CardIndex {
    card_index("f6a6da20-52c8-4921-9884-29d3a3051b0d")
}

/// Martyrdom ({1}{W}{W} Instant, Partial):
/// Grants target creature you control "{0}: Prevent the next 1 damage that would be dealt to any target this turn" until end of turn.
#[test]
fn martyrdom_grants_free_damage_prevention_ability_to_target_creature_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), mountain(), grizzly_bears()],
        )
        .hand(0, &[martyrdom(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, martyrdom());
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

    // Activate the granted {0} ability targeting p1, then deal 3 damage with Lightning Bolt.
    let grant = crate::effects::granted_activated(engine.state(), bears)
        .next()
        .unwrap()
        .id;
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bears,
                ability_index: grant,
            },
        )
        .unwrap();
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

    let life_before = life(&engine, p1);
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

    assert_eq!(life(&engine, p1), life_before - 2);
}
