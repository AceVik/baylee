#[allow(clippy::wildcard_imports)]
use super::*;

fn quicksilver_amulet() -> CardIndex {
    card_index("e1096a98-a631-43f5-97d8-325001d608b3")
}

#[test]
fn quicksilver_amulet_puts_creature_from_hand_onto_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), quicksilver_amulet()],
        )
        .hand(0, &[earth_elemental()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let amulet = on_battlefield(&engine, p0, quicksilver_amulet()).unwrap();
    let elem = in_hand(&engine, p0, earth_elemental()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: amulet,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![elem],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, earth_elemental()).is_some());
}
