//! `cards/lands/caves/captivating_cave.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Captivating Cave prints `{T}: Add {C}`, `{1}, {T}: Add one mana of any color`, and the sorcery
/// speed `{4}, {T}, Sacrifice this land: Put two +1/+1 counters on target creature.`
/// The card is marked `Coverage::Implemented`.
/// With the board tapped out for five green — four Forests and the Llanowar Elves, which is a
/// mana source like any other — announcing ability index 2 requires picking a target creature
/// before paying costs; once targeted, the cave is sacrificed to the graveyard and resolution
/// grants two `CounterKind::P1P1` counters to the target.
#[test]
fn captivating_cave_sacrifices_to_put_two_counters_on_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), llanowar_elves()],
        )
        .hand(0, &[captivating_cave()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let _cave = play_land(&mut engine, p0, captivating_cave());
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf exists");
    assert_eq!(pt(&engine, elf), (1, 1));

    tap_all_mana_but(&mut engine, p0, Some(captivating_cave()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        5,
        "four Forests and the Elf, which taps for green as well"
    );

    activate(&mut engine, p0, captivating_cave(), 2);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice");
    };
    assert!(options.contains(&elf));
    assert!(on_battlefield(&engine, p0, captivating_cave()).is_some());

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();

    assert!(in_graveyard(&engine, p0, captivating_cave()).is_some());

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, elf, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, elf), (3, 3));
}

/// Forgotten Monument prints `{{T}}: Add {{C}}` and `Other Caves you control have "{{T}}, Pay 1 life: Add one mana of any color."`
/// Under `Coverage::Implemented`, both abilities are fully supported. This test verifies that Forgotten
/// Monument taps for colorless mana without life loss, grants the activated mana ability to another controlled Cave
/// (`captivating_cave()`) which pays 1 life and adds mana of any color, and confirms that Forgotten Monument does not
/// grant the ability to itself, to non-Cave lands, or to an opponent's Cave.
#[test]
fn forgotten_monument_grants_mana_ability_to_other_caves_you_control() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forgotten_monument(), captivating_cave(), forest()])
        .battlefield(1, &[cavernous_maw()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let monument =
        on_battlefield(&engine, p0, forgotten_monument()).expect("monument on battlefield");
    let cave =
        on_battlefield(&engine, p0, captivating_cave()).expect("captivating cave on battlefield");
    let my_forest = on_battlefield(&engine, p0, forest()).expect("forest on battlefield");
    let opp_cave =
        on_battlefield(&engine, p1, cavernous_maw()).expect("opponent cave on battlefield");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };

    assert!(
        legal
            .abilities
            .contains(&(cave, crate::choice::GRANTED_ABILITY)),
        "another controlled Cave receives the granted ability"
    );
    assert!(
        !legal
            .abilities
            .contains(&(monument, crate::choice::GRANTED_ABILITY)),
        "Forgotten Monument does not grant the ability to itself"
    );
    assert!(
        !legal
            .abilities
            .contains(&(my_forest, crate::choice::GRANTED_ABILITY)),
        "non-Cave land does not receive the granted ability"
    );

    // Forgotten Monument taps for colorless mana without life loss.
    activate(&mut engine, p0, forgotten_monument(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "monument produces {{C}}"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "tapping monument does not cost life"
    );
    assert!(is_tapped(&engine, monument), "monument is tapped");

    // Activating the granted ability on the other Cave costs {T} and 1 life, and adds any color.
    activate(
        &mut engine,
        p0,
        captivating_cave(),
        crate::choice::GRANTED_ABILITY,
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "paying 1 life for the granted ability"
    );
    assert!(is_tapped(&engine, cave), "cave is tapped");

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5, "grants mana of any color");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "added one red mana"
    );
    assert!(
        stack_is_empty(&engine),
        "mana ability resolves without using the stack"
    );

    // Verify that on the opponent's turn, their Cave does not have the granted ability.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p1
    });
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority for p1, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, index)| *id == opp_cave && *index == crate::choice::GRANTED_ABILITY),
        "opponent Cave does not receive the grant"
    );
}
