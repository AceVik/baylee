//! `cards/creatures/artifacts/mv_0/ornithopter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ornithopter prints a `{0}` cost, an Artifact Creature body of 0/2 and one
/// word of rules text: flying. The scenario reads all three off one game — the
/// cast happens out of an **empty** pool with the board's only Forest still
/// standing, so `{0}` is the card's doing and not a land's — and the keyword is
/// read where the rules actually use it: the attack declaration offers the
/// Thopter, and the block pairing across the table offers p1's own flying
/// creature while leaving out p1's ground Elf, both untapped and under the same
/// seat. Those two differ in nothing the engine may act on but the keyword, so
/// a pairing holding both or neither is exactly what a missing flying check
/// would look like — the second Ornithopter is across the table on purpose,
/// because its flying is the one this test has already read.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn ornithopter_lands_for_free_and_only_a_flier_may_block_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[ornithopter()])
        .battlefield(1, &[ornithopter(), llanowar_elves()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, ornithopter()).expect("the Thopter is in hand");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is on the table");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats and nothing has been tapped, which is the pool a {{0}} \
         spell has to be castable out of"
    );
    assert!(
        legal.castable.contains(&card),
        "the printed {{0}} is affordable on an empty pool: {:?}",
        legal.castable
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the printed {{0}} is the whole cost");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let thopter = on_battlefield(&engine, p0, ornithopter()).expect("the Thopter resolved");
    assert!(
        !is_tapped(&engine, land),
        "a {{0}} spell spends no mana, so the only land on the board never moved"
    );
    let kinds = types(&engine, thopter);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::CREATURE),
        "\"Artifact Creature\" is the type line it arrives with: {kinds:?}"
    );
    assert_eq!(pt(&engine, thopter), (0, 2), "and the body the card prints");
    assert!(
        keywords(&engine, thopter).contains(KeywordSet::FLYING),
        "flying is its whole rules text"
    );

    // CR 302.6: a creature that entered this turn cannot attack, so the
    // scenario waits for a turn of p0's to have begun — which is also what
    // untaps the two creatures across the table.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the turn belongs to the Thopter's controller");
    assert!(
        attackers.contains(&thopter),
        "an untapped 0/2 does not stop being a creature because it is an \
         artifact, nor because its power is nothing: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(thopter, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p1, "the seat being attacked answers the pairing");
    let wings = on_battlefield(&engine, p1, ornithopter()).expect("their Thopter is out");
    let grounder = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let can_block = |blocker: ObjectId| {
        blockers
            .iter()
            .any(|o| o.blocker == blocker && o.attackers.contains(&thopter))
    };
    assert!(
        can_block(wings),
        "a creature with flying may block a flier: {blockers:?}"
    );
    assert!(
        !can_block(grounder),
        "and one without may not, though it stands untapped beside the first \
         and under the same seat: {blockers:?}"
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        20,
        "nobody blocked it, so the 0/2 connected — and took nothing with it, \
         which is the printed power and not a size a keyword added"
    );
}
