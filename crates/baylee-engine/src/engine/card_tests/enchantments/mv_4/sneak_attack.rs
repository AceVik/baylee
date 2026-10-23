#[allow(clippy::wildcard_imports)]
use super::*;

fn sneak_attack() -> CardIndex {
    card_index("e7aee670-e6ac-4b3a-b3d1-7b7013db8f3d")
}

#[test]
fn sneak_attack_puts_creature_from_hand_onto_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[gray_ogre()])
        .battlefield(0, &[sneak_attack(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre_id = in_hand(&engine, p0, gray_ogre()).expect("ogre in hand");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, sneak_attack(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![ogre_id],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
}
