//! `cards/creatures/mv_4/ridgetop_raptor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ridgetop Raptor is a printed 2/1 whose whole rules text is double strike,
/// and no reading of a keyword can tell that from a bigger body — only combat
/// can. So the Raptor attacks an empty board across the table and the
/// defending seat's life is read twice: once with the first-strike damage step
/// behind it, where a 2/1 has dealt 2, and again past the regular damage step,
/// where the same creature has dealt 2 more. A hypothetical single 4-damage
/// event would land on the same final total and never show the intermediate
/// eighteen, which is exactly what the two readings separate.
#[test]
fn ridgetop_raptor_strikes_in_both_combat_damage_steps_for_four_from_two_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // The Raptor is seated rather than cast: what this test reads is combat,
    // and a creature cast this turn would be summoning sick (CR 302.6) and
    // never on the attack offer at all.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ridgetop_raptor()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let raptor = on_battlefield(&engine, p0, ridgetop_raptor()).expect("the Raptor is out");
    assert_eq!(pt(&engine, raptor), (2, 1), "the body the card prints");
    assert!(
        keywords(&engine, raptor).contains(KeywordSet::DOUBLE_STRIKE),
        "the one printed line reaches the permanent through the layers"
    );

    // The declaration is a reading in itself: an attacker the engine does not
    // enumerate cannot be declared, and a creature with summoning sickness is
    // not enumerated.
    let blockers = attack_and_collect_blocks(&mut engine, raptor, p1);
    assert!(
        blockers.is_empty(),
        "nothing across the table can block, so both damage steps belong to \
         the Raptor alone: {blockers:?}"
    );
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        panic!(
            "expected the declare-blockers question, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declaring no blockers is always legal");

    // The first-strike damage step. Eighteen is the only life total that comes
    // from *one* of the two strikes: a plain 2/1 would never show it, and a
    // 4-damage event would skip straight past it.
    pass_until(&mut engine, |e| e.state().players[1].life == 18);
    assert!(
        on_battlefield(&engine, p0, ridgetop_raptor()).is_some(),
        "damage to a player kills nothing, so the Raptor is still in combat"
    );

    // Past the regular damage step, where the same two power lands a second
    // time. The end step is the predicate rather than an empty stack, because
    // the stack is already empty the moment attackers are declared.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        16,
        "2 damage in each of the two steps double strike gives a 2/1 — a single \
         4-damage event would have skipped the eighteen above, and one strike \
         alone would have stopped there"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the defending seat, not to the attacker"
    );
}
