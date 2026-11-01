//! `cards/sorceries/mv_3/repentance.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn repentance() -> CardIndex {
    card_index("9a345a67-0c6c-4f6f-b3c9-d525199cdbfa")
}

/// `Repentance` ({2}{W}, Sorcery, `Coverage::Implemented`):
/// `Target creature deals damage to itself equal to its power.`
#[test]
fn repentance_causes_target_creature_to_deal_damage_equal_to_its_power_to_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[repentance()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, repentance());
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

    // 3/3 Hill Giant dealt 3 damage to itself and died.
    assert!(on_battlefield(&engine, p1, HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, p1, HILL_GIANT).is_some());
}
