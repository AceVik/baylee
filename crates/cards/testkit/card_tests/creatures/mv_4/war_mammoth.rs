//! `cards/creatures/mv_4/war_mammoth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// War Mammoth is a `{3}{G}` 3/3 Elephant whose whole printed text is
/// "Trample", so nothing short of a real combat step tells it from a vanilla
/// 3/3. The scenario casts it off four Forests — the pool reads empty
/// afterwards, so the `{3}{G}` was really paid — waits a turn for summoning
/// sickness (CR 302.6), and then attacks into a 1/1 Llanowar Elves. One of the
/// three damage is lethal to the blocker and the other two must trample over
/// (CR 702.19b): a defender at 18 is the only number that says trample is a
/// rule here and not a keyword sitting on a card.
#[test]
fn war_mammoth_casts_for_four_and_tramples_two_over_a_one_one_blocker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[war_mammoth()])
        .battlefield(1, &[llanowar_elves()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Forests are the whole board and the whole cost: the pool is empty
    // once the Mammoth has landed, so the `{3}{G}` was paid rather than skipped.
    cast_from_hand(&mut engine, p0, war_mammoth());
    pass_until(&mut engine, stack_is_empty);
    let mammoth = on_battlefield(&engine, p0, war_mammoth()).expect("the Mammoth resolved");
    assert_eq!(pt(&engine, mammoth), (3, 3), "the printed 3/3 body");
    assert!(
        keywords(&engine, mammoth).contains(KeywordSet::TRAMPLE),
        "the one line of rules text the card prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "four Forests paid {{3}}{{G}} to the last mana"
    );

    // It entered this turn as a spell, so CR 302.6 keeps it out of combat
    // until its controller's next turn has begun.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is still out");

    let blocks = attack_and_collect_blocks(&mut engine, mammoth, p1);
    assert!(
        blocks
            .iter()
            .any(|option| option.blocker == elf && option.attackers.contains(&mammoth)),
        "the 1/1 may stand in front of the 3/3: trample changes what the damage \
         does, not who may block it: {blocks:?}"
    );
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        panic!(
            "expected the declare-blockers question, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            player,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, mammoth)],
            },
        )
        .expect("the Elf was one of the pairings the engine offered");

    // The end step is past the combat damage step (CR 510.2); `stack_is_empty`
    // would stop the walk while the damage was still unassigned.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one of the three damage is lethal to a printed 1/1 (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        18,
        "and the remaining two trample over: 20 would mean the Elf ate all \
         three points, 17 that it was never in the way (CR 702.19b)"
    );
    assert!(
        on_battlefield(&engine, p0, war_mammoth()).is_some(),
        "the Mammoth survives the combat it started"
    );
}
