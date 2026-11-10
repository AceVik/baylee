//! `cards/enchantments/classes/mv_2/druid_class.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Druid Class`: "Landfall — Whenever a land you control enters, you gain 1
/// life. `{{2}}{{G}}`: Level 2. You may play an additional land on each of
/// your turns. `{{4}}{{G}}`: Level 3. When this Class becomes level 3, target
/// land you control becomes a creature with haste and 'This creature's power
/// and toughness are each equal to the number of lands you control.' It's
/// still a land."
///
/// Played end to end: a land gains 1 life; at level 1 a second land is not
/// offered; at level 2 it is. Gaining level 3 finishes before its trigger
/// asks for a target — the Class holds two level counters while the
/// question is open, so a target lost in response could not take the level
/// with it. The chosen Forest is a land creature with haste, 10/10 with
/// ten lands, and 11/11 once the next turn's land arrives.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn druid_class_gains_a_land_drop_at_level_two_and_animates_a_land_at_level_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(102, forest())
        .battlefield(
            0,
            &[
                druid_class(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lands_offered = |engine: &Engine<RegistryLookup>| {
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        legal.lands.len()
    };

    assert_eq!(engine.state().players[0].life, 20, "starts at 20 life");
    let class = on_battlefield(&engine, p0, druid_class()).expect("Druid Class on battlefield");
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 21, "Landfall gains 1 life");
    assert_eq!(lands_offered(&engine), 0, "level 1: one land a turn");

    tap_mana_except(&mut engine, p0, class);
    assert_eq!(engine.state().players[0].mana_pool.total(), 9);
    // Ability 1 is `{{2}}{{G}}: Level 2`.
    activate(&mut engine, p0, druid_class(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, class, CounterKind::Level),
        1,
        "level 2"
    );
    assert!(
        lands_offered(&engine) > 0,
        "level 2: \"you may play an additional land\""
    );
    let second = play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 22, "and Landfall again");

    // Ability 3 is `{{4}}{{G}}: Level 3`.
    activate(&mut engine, p0, druid_class(), 3);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert_eq!(
        counters_on(&engine, class, CounterKind::Level),
        2,
        "the Class is level 3 before its trigger asks for a target"
    );
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&second), "a land you control: {options:?}");
    assert!(!options.contains(&class), "and only a land");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![second],
                players: vec![],
            },
        )
        .expect("the Forest was offered");
    pass_until(&mut engine, stack_is_empty);

    let now = types(&engine, second);
    assert!(
        now.contains(TypeSet::CREATURE) && now.contains(TypeSet::LAND),
        "a creature, and still a land: {now:?}"
    );
    assert!(keywords(&engine, second).contains(KeywordSet::HASTE));
    assert_eq!(pt(&engine, second), (10, 10), "ten lands you control");

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(walk_to_own_main(&mut engine, p0), "p0's next turn comes");
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, second),
        (11, 11),
        "the count is read as it changes, and the effect outlasts the turn"
    );
}
