//! `cards/creatures/mv_3/blistering_barrier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blistering Barrier prints `{2}{R}` for a 5/2 Wall with defender: “This
/// creature can't attack.” That one line is the entire card profile, so it is
/// read during the combat step and not on the body: a Llanowar Elves stands
/// untapped next to it and is the control for the `attackers` offer actually
/// naming attackers — and the Barrier is not included in the *same* offer,
/// even though the test advances a full turn cycle first so that summoning
/// sickness (CR 302.6) doesn't play the role of the defender.
#[test]
fn blistering_barrier_is_a_five_two_defender_never_offered_as_an_attacker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[blistering_barrier()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Three Mountains produce {2}{R}. The Elf is held back by name because
    // `tap_all_mana` would also take its printed `{T}: Add {G}` (#159) — and
    // a creature tapped for mana would no longer be allowed to attack below.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        3,
        "three Mountains, three red — the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, blistering_barrier());
    pass_until(&mut engine, stack_is_empty);

    let barrier = on_battlefield(&engine, p0, blistering_barrier()).expect("die Barriere landete");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("der Elf steht noch");
    assert_eq!(pt(&engine, barrier), (5, 2), "der gedruckte 5/2-Körper");
    assert!(
        types(&engine, barrier).contains(TypeSet::CREATURE),
        "and what landed is a creature: {:?}",
        types(&engine, barrier)
    );
    assert!(
        keywords(&engine, barrier).contains(KeywordSet::DEFENDER),
        "with Defender on the projected card"
    );

    // A full turn cycle before the attack is read: the Barrier is
    // then no longer summoning sick, so "not offered" is a statement
    // about the printed text and not about the arrival turn.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until bleibt nur auf der Angriffserklärung stehen")
    };
    assert!(
        attackers.contains(&elf),
        "the untapped Elf without its own text is the control: {attackers:?}"
    );
    assert!(
        !attackers.contains(&barrier),
        "and the 5/2 with Defender is not in the same offer: {attackers:?}"
    );
}
