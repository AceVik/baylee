//! `cards/sorceries/mv_2/infernal_harvest.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn infernal_harvest() -> CardIndex {
    card_index("b8c8cde9-34db-4185-86f5-4289969880c5")
}

/// `Infernal Harvest` ({1}{B}, Sorcery, `Coverage::Partial`) returns a Swamp
/// you control to its owner's hand and deals 1 damage to target creature.
#[test]
fn infernal_harvest_returns_swamp_to_hand_and_deals_one_damage_to_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[infernal_harvest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, infernal_harvest());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, swamp()).is_some());
    assert_eq!(engine.state().object(bear).unwrap().damage, 1);
}
