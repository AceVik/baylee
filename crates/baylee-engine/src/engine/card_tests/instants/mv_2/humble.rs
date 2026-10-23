#[allow(clippy::wildcard_imports)]
use super::*;

fn humble() -> CardIndex {
    card_index("d2d41f12-18d8-473b-9308-3f02d7d791f0")
}

#[test]
fn humble_removes_abilities_and_sets_target_creature_to_0_1() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[humble()])
        .battlefield(0, &[plains(), plains(), llanowar_elves(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");
    assert!(!is_tapped(&engine, bear));
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority before Humble");
    };
    assert!(
        legal.abilities.contains(&(bear, 0)),
        "the untapped Elf can make mana before Humble"
    );
    tap_mana_except(&mut engine, p0, bear);
    assert!(
        !is_tapped(&engine, bear),
        "the Elf is not spent paying for Humble"
    );
    cast_with_floating(&mut engine, p0, humble());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, bear);
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (0, 1));
    assert!(
        !is_tapped(&engine, bear),
        "ability loss is independent of tapping"
    );
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority");
    };
    assert!(!legal.abilities.iter().any(|(id, _)| *id == bear));
    let other = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, other), (2, 2));
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(pt(&engine, bear), (1, 1));
    activate(&mut engine, p0, llanowar_elves(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
}
