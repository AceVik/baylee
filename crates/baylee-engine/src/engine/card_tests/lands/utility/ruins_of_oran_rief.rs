//! `cards/lands/utility/ruins_of_oran_rief.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ruins of Oran-Rief enters tapped and prints `{T}: Add {C}`; the second
/// printed sentence — a +1/+1 counter on a colorless creature that entered
/// this turn — has no DSL filter (`Coverage::Partial`), so nothing here
/// presses it. The land has to be *played* rather than seeded on a starting
/// battlefield: `SeatSpec::starting_battlefield` places a permanent with
/// `Cause::Setup`, which is a placement and not an entry, so a land arriving
/// that way is untapped whatever it prints. Only a real `PlayLand` reaches
/// `EnterModifier::Tapped`, and only a later turn's untap step can stand the
/// land back up to pay for its own `{T}`.
#[test]
fn ruins_of_oran_rief_enters_tapped_and_taps_for_one_colorless() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(7171, forest())
        .hand(0, &[ruins_of_oran_rief()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, ruins_of_oran_rief());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — and it entered by being played, so the \
         entry modifier is what tapped it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a land that just entered tapped has paid for nothing and made nothing"
    );

    // A whole turn has to pass: a land that entered tapped cannot pay for its
    // own `{T}`, so the untap step is the only thing that makes the printed
    // mana ability reachable at all.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it back up, which is the control the tap below \
         needs: otherwise it would only show that a tapped land stays tapped"
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 1,
        "the land's printed {{T}} is the only mana route on this board"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}} puts one colorless mana in the pool"
    );
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}

/// Ruins of Oran-Rief prints `This land enters tapped.`, `{{T}}: Add {{C}}.`, and
/// `{{T}}: Put a +1/+1 counter on target colorless creature that entered this turn.`
/// Under `Coverage::Implemented`, both abilities and the entry modifier are supported.
/// This test plays the land tapped, passes to the next turn so it untaps, floats mana to cast
/// a colorless creature (`myr_retriever()`), activates ability 1 targeting the newly entered
/// colorless creature to place a +1/+1 counter, verifies on the subsequent turn that the creature
/// is no longer a valid target, and activates ability 0 to produce colorless mana.
#[test]
fn ruins_of_oran_rief_enters_tapped_and_puts_counter_on_arrived_colorless_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[ruins_of_oran_rief(), myr_retriever()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, ruins_of_oran_rief());
    assert!(
        entered_tapped(&engine, land),
        "Ruins of Oran-Rief enters tapped"
    );

    // Pass turn to untap.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "land untaps on next turn");

    // Before any creature entered this turn, ability 1 has no legal targets and is withheld.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == land && *idx == 1),
        "ability 1 is withheld when no colorless creature entered this turn"
    );

    // Float mana keeping Ruins of Oran-Rief untapped, then cast Myr Retriever.
    let myr = in_hand(&engine, p0, myr_retriever()).expect("myr in hand");
    tap_all_mana_but(&mut engine, p0, Some(ruins_of_oran_rief()));
    cast_with_floating(&mut engine, p0, myr_retriever());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, myr), (1, 1), "Myr Retriever is a 1/1");

    // Ability 1 is now offered.
    activate(&mut engine, p0, ruins_of_oran_rief(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![myr],
        "the colorless creature that entered this turn is the sole target"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![myr] })
        .expect("targeting the entered colorless creature is legal");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, myr, CounterKind::P1P1),
        1,
        "gains a +1/+1 counter"
    );
    assert_eq!(pt(&engine, myr), (2, 2), "grows to a 2/2");
    assert!(is_tapped(&engine, land), "land tapped for ability 1");

    // On the following turn, Myr Retriever did not enter this turn and cannot be targeted.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "land untaps");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, idx)| *id == land && *idx == 1),
        "ability 1 is withheld because the creature entered on an earlier turn"
    );

    // Ability 0 taps for colorless mana.
    activate(&mut engine, p0, ruins_of_oran_rief(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "ability 0 adds {{C}}"
    );
    assert!(is_tapped(&engine, land), "land tapped for ability 0");
}
