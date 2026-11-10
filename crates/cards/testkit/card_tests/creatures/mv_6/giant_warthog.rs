//! `cards/creatures/mv_6/giant_warthog.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Giant Warthog — {5}{G}, Creature — Boar Beast, 5/5, trample. The card is a
/// body and one keyword, so the test plays both in a real combat step rather
/// than reading them off the permanent: six Forests pay {5}{G} out of one main
/// phase, and the swine has to wait a turn (CR 302.6) before it may attack at
/// all. A printed 1/1 blocks it, and trample is what turns that block into a
/// dead Elf *and* four damage to the defending player — one damage assigned to
/// the blocker, the four overkill trampled on (CR 702.19b). Without the
/// keyword the same declared block would leave that player at twenty.
#[test]
fn giant_warthog_tramples_four_damage_over_a_one_one_blocker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[giant_warthog()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {5}{G} out of six Forests: six tapped sources, six mana, and a pool the
    // cast empties — nothing else on this board could have paid for the Boar.
    cast_from_hand(&mut engine, p0, giant_warthog());
    pass_until(&mut engine, stack_is_empty);
    let boar = on_battlefield(&engine, p0, giant_warthog()).expect("the Warthog resolved");
    assert_eq!(pt(&engine, boar), (5, 5), "the printed body");
    assert!(
        keywords(&engine, boar).contains(KeywordSet::TRAMPLE),
        "and the printed trample"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "six Forests paid the {{5}}{{G}} to the last mana"
    );

    // A creature that arrived this turn has summoning sickness (CR 302.6), so
    // the combat below is a turn later: through the Elf's seat and back.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is still out");

    let blocks = attack_and_collect_blocks(&mut engine, boar, p1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == elf && b.attackers.contains(&boar)),
        "a 1/1 may block the trampler: {blocks:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, boar)],
            },
        )
        .expect("the pairing the engine offered is the one it accepts");

    // `stack_is_empty` would stop before the combat damage step, so the walk
    // goes to the ending phase, which is past it (CR 510.2).
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage is lethal to a printed 1/1 (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        16,
        "and the other four damage trample over to the player the Warthog attacked"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the Elf's whole body came back at nothing: 1/1 against 5/5"
    );
    assert!(
        on_battlefield(&engine, p0, giant_warthog()).is_some(),
        "the Warthog survives its own attack and the block that met it"
    );
}
