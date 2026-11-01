//! `cards/enchantments/mv_3/samite_sanctuary.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn samite_sanctuary() -> CardIndex {
    card_index("9e6e1ea6-4682-4056-b70e-fe45d0c9fdee")
}

/// Samite Sanctuary ({2}{W} Enchantment):
/// "{2}: Prevent the next 1 damage that would be dealt to target creature this turn. Any player may activate this ability."
#[test]
fn samite_sanctuary_prevents_next_one_damage_when_activated_by_any_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = samite_sanctuary();
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[subject, mountain()])
        .battlefield(1, &[index::HILL_GIANT, forest(), forest()])
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sanctuary = on_battlefield(&engine, p0, subject).unwrap();
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, index::LIGHTNING_BOLT);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: sanctuary,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, index::HILL_GIANT).is_some(),
        "Preventing 1 of Lightning Bolt's 3 damage leaves the 3/3 Hill Giant alive with 2 damage"
    );
    assert_eq!(engine.state().object(giant).unwrap().damage, 2);
}
