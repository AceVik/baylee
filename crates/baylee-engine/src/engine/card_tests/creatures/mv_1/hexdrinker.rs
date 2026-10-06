//! `cards/creatures/mv_1/hexdrinker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Hexdrinker` prints `Level up {{1}}`, `LEVEL 3-7: 4/4, Protection from
/// instants` and `LEVEL 8+: 6/6, Protection from everything`.
///
/// Three level-ups, each a real activation paid with `{{1}}`: the second
/// leaves the printed 2/1, the third crosses into the first band (CR
/// 711.2a) and makes it a 4/4. An opponent's Lightning Bolt is then offered
/// the Elves and not the Snake, and the same opponent's Maelstrom Pulse, a
/// sorcery, is offered the Snake and destroys it: the band's protection is
/// from instants and nothing wider.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn hexdrinker_is_a_four_four_with_protection_from_instants_from_level_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[hexdrinker(), forest(), forest(), forest(), llanowar_elves()],
        )
        .battlefield(1, &[mountain(), swamp(), forest(), forest()])
        .hand(1, &[lightning_bolt(), maelstrom_pulse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let snake = on_battlefield(&engine, p0, hexdrinker()).expect("hexdrinker seated");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves beside it");
    assert_eq!(pt(&engine, snake), (2, 1));

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    for (level, body) in [(1, (2, 1)), (2, (2, 1)), (3, (4, 4))] {
        activate(&mut engine, p0, hexdrinker(), 0);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(counters_on(&engine, snake, CounterKind::Level), level);
        assert_eq!(pt(&engine, snake), body, "at level {level}");
    }
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "each level cost its {{1}}"
    );

    reach_their_main_phase(&mut engine, p1);
    let mountain_id = on_battlefield(&engine, p1, mountain()).expect("p1's Mountain");
    tap_mana_where(&mut engine, p1, |id| id == mountain_id);
    cast_with_floating(&mut engine, p1, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the Bolt asks for a target, got {:?}", engine.pending())
    };
    assert!(options.contains(&elves), "the Elves may be Bolted");
    assert!(
        !options.contains(&snake),
        "a level-3 Hexdrinker has protection from instants"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .expect("the Elves were offered");
    pass_until(&mut engine, stack_is_empty);

    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, maelstrom_pulse());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the Pulse asks for a target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&snake),
        "a sorcery is not an instant, so the Pulse may target it"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![snake],
                players: vec![],
            },
        )
        .expect("the Snake was offered");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, hexdrinker()).is_none(),
        "and destroys it"
    );
}

/// Hexdrinker's second band: "LEVEL 8+: 6/6, Protection from everything"
/// (CR 711.2b, 702.16j).
///
/// It starts at level 6 (the harness plants the counters) and is equipped
/// with a Bonesplitter, a 6/4. The seventh level leaves it in the first
/// band; the eighth makes it a 6/6 and protection from everything makes the
/// Equipment's attachment illegal: the Bonesplitter comes off and stays on
/// the battlefield (CR 702.16d), so the body is 6/6 and not 8/6. Equipping
/// it again is offered only the Elves beside it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn hexdrinker_at_level_eight_is_a_six_six_that_sheds_its_equipment() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                hexdrinker(),
                hexdrinker_s_bonesplitter(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .start();
    let snake = on_battlefield(&engine, p0, hexdrinker()).expect("hexdrinker seated");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        crate::replacement::put_counters(state, snake, CounterKind::Level, 6);
    }
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let splitter =
        on_battlefield(&engine, p0, hexdrinker_s_bonesplitter()).expect("the Equipment is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves beside it");
    assert_eq!(pt(&engine, snake), (4, 4), "level 6 is in the first band");

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    let equip = |engine: &Engine<RegistryLookup>| {
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        legal
            .abilities
            .iter()
            .copied()
            .find(|(src, _)| *src == splitter)
            .expect("Equip {1} is offered while there is a creature to wear it")
    };
    let (source, ability_index) = equip(&engine);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the equip is paid from the pool");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![snake],
            },
        )
        .expect("a level-6 Hexdrinker may be equipped");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, snake), (6, 4), "a 4/4 with +2/+0");

    activate(&mut engine, p0, hexdrinker(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, snake, CounterKind::Level), 7);
    assert_eq!(
        pt(&engine, snake),
        (6, 4),
        "level 7 is still the first band"
    );

    activate(&mut engine, p0, hexdrinker(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, snake, CounterKind::Level), 8);
    assert_eq!(
        engine.state().object(splitter).and_then(|o| o.attached_to),
        None,
        "protection from everything unattaches the Equipment"
    );
    assert!(
        on_battlefield(&engine, p0, hexdrinker_s_bonesplitter()).is_some(),
        "which stays on the battlefield"
    );
    assert_eq!(pt(&engine, snake), (6, 6), "a 6/6 with nothing on it");

    let (source, ability_index) = equip(&engine);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the last Forest pays the equip");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("equip asks for a creature, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![elves],
        "a level-8 Hexdrinker can't be targeted by anything"
    );
}
