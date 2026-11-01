#[allow(clippy::wildcard_imports)]
use super::*;

fn fiery_mantle() -> CardIndex {
    card_index("1b553ed4-12eb-4a53-8e1f-d47ec92b1d65")
}

#[test]
fn fiery_mantle_pumps_enchanted_creature_and_returns_to_hand_on_death() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[fiery_mantle()])
        .battlefield(0, &[mountain(), mountain(), mountain(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, fiery_mantle());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, bear);
    }
    pass_until(&mut engine, stack_is_empty);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, fiery_mantle(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bear), (3, 2));

    let aura = on_battlefield(&engine, p0, fiery_mantle()).expect("attached");
    kill(&mut engine, aura);
    assert!(in_hand(&engine, p0, fiery_mantle()).is_some());
}

#[test]
fn fiery_mantle_does_not_return_when_discarded_from_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[fiery_mantle()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    hand_to_graveyard(&mut engine, p0, fiery_mantle());
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, fiery_mantle()).is_some());
    assert!(in_hand(&engine, p0, fiery_mantle()).is_none());
}
