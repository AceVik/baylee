//! `cards/creatures/mv_2/vine_trellis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vine Trellis — {1}{G}, a 0/4 Plant Wall with "Defender" and
/// "{T}: Add {G}". Both printed lines are the test, and both need a control:
/// the untapped Llanowar Elves next to it may attack, the Trellis may not —
/// an empty attack declaration would also be fulfilled by a combat step that
/// never took place. It only becomes tapped on the next own turn, because a
/// creature that has only just arrived does not even offer its {T}
/// (CR 302.6); there the pool is empty, which makes "exactly one green" the
/// exact statement. That the ability stands in `legal.abilities` and not in
/// the CR-305.6 shorthand is the difference between a printed `{T}: Add {G}`
/// and a basic land type.
#[test]
fn vine_trellis_defends_and_taps_for_green_on_a_later_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves()])
        .hand(0, &[vine_trellis()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 erreicht seinen Main");

    // The Elf is exempted: `tap_all_mana` would have tapped its own
    // `{T}: Add {G}` along with it and thereby removed it from combat (#159).
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests, and the Elves paid nothing extra"
    );
    cast_with_floating(&mut engine, p0, vine_trellis());
    pass_until(&mut engine, stack_is_empty);

    let trellis = on_battlefield(&engine, p0, vine_trellis()).expect("die Trellis ist angekommen");
    assert_eq!(pt(&engine, trellis), (0, 4), "der gedruckte 0/4-Körper");
    assert!(
        types(&engine, trellis).contains(TypeSet::CREATURE),
        "and it is the creature that it prints: {:?}",
        types(&engine, trellis)
    );
    assert!(
        keywords(&engine, trellis).contains(KeywordSet::DEFENDER),
        "Defender erreicht das Permanent über das Layer-System"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "das {{1}}{{G}} ist ausgegeben"
    );

    // Attack: the untapped Elf is in, the Wall is not.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until hält auf nichts außer der Angriffserklärung")
    };
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("der Elf steht noch");
    assert!(
        attackers.contains(&elves),
        "the control: an untapped 1/1 may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&trellis),
        "\"This creature can't attack\" — the 0/4 Wall is not in the offer: {attackers:?}"
    );

    // A full turn around: only then is the {T} of the creature payable.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no mana carries over into the new turn (CR 500.5)"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected: priority, got: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(trellis, 0)),
        "a printed `{{T}}: Add {{G}}` is an ability with an index and \
         therefore appears in `abilities`: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&trellis),
        "and not in the CR 305.6 shortcut, which carries basic land types and \
         granted mana abilities and nothing else: {:?}",
        legal.mana_abilities
    );

    activate(&mut engine, p0, vine_trellis(), 0);
    assert!(is_tapped(&engine, trellis), "{{T}} wurde bezahlt");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability does not use the stack"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "{{T}}: Add {{G}}");
    assert_eq!(pool.total(), 1, "ein Mana, und sonst nichts");
}
