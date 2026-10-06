//! `cards/creatures/mv_6/rorix_bladewing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rorix Bladewing is `{3}{R}{R}{R}` for a 6/5 legendary Dragon whose whole
/// text is flying and haste, so a test has to reach a declare-attackers step
/// on the very turn it lands — a creature that entered this turn is offered
/// as an attacker only because of haste, and the six damage that arrives on
/// the other seat is the same claim read off a life total. Flying is read off
/// the same combat step from the other side: a ground creature across the
/// table may not be paired with it. The six Mountains are tapped before
/// anything is claimed about the cast, because the offer is read off the mana
/// pool and not off the untapped lands.
#[test]
fn rorix_bladewing_lands_as_a_six_five_flier_that_attacks_the_turn_it_arrives() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 6])
        .hand(0, &[rorix_bladewing()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Mountains and nothing else on the board: six red pays {{3}}{{R}}{{R}}{{R}}"
    );
    cast_with_floating(&mut engine, p0, rorix_bladewing());
    pass_until(&mut engine, stack_is_empty);

    let rorix = on_battlefield(&engine, p0, rorix_bladewing()).expect("Rorix resolved");
    let elf =
        on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf stands across the table");
    assert_eq!(pt(&engine, rorix), (6, 5), "the body the card prints");
    assert!(
        types(&engine, rorix).contains(TypeSet::CREATURE),
        "and it is a creature, not a permanent the layers left without a type: {:?}",
        types(&engine, rorix)
    );
    let granted = keywords(&engine, rorix);
    assert!(granted.contains(KeywordSet::FLYING), "Flying");
    assert!(granted.contains(KeywordSet::HASTE), "and haste");

    // Haste is read where it is worth something: the declaration this turn.
    // `attack_and_collect_blocks` panics if the creature is not on the offer,
    // which is exactly what a summoning-sick creature would be missing.
    let blocks = attack_and_collect_blocks(&mut engine, rorix, p1);
    assert!(
        !blocks
            .iter()
            .any(|option| option.blocker == elf && option.attackers.contains(&rorix)),
        "flying: a ground creature is not paired with it — {blocks:?}"
    );

    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        panic!(
            "the attack declaration leads to the blockers, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declaring no blockers is always legal");
    // Not `stack_is_empty`: the stack is already empty the moment blockers are
    // declared, so the damage step (CR 510) has not happened yet. The end step
    // is past it and is what the life total below needs.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        14,
        "a 6/5 that attacked the turn it entered: six damage, which a creature \
         without haste could not have dealt here at all"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the Elf never blocked and never moved"
    );
    assert!(
        on_battlefield(&engine, p0, rorix_bladewing()).is_some(),
        "and the Dragon is still standing after combat"
    );
}
