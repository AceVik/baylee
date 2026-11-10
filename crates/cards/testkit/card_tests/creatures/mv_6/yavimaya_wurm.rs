//! `cards/creatures/mv_6/yavimaya_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Yavimaya Wurm — {4}{G}{G} — 6/4 Wurm with trample. The card is one word of
/// rules text on one body, so the scenario plays both halves: six Forests pay
/// the printed six exactly and the Wurm arrives by being cast, reading as the
/// 6/4 the card prints with the keyword projected onto the permanent. Trample
/// only means anything once something blocks, so the creature is turned
/// sideways a turn later — a creature cast this turn may not attack at all
/// (CR 302.6) — into a printed 1/1: one damage is lethal to the blocker
/// (CR 510.1c) and the other five spill onto the defending player, while the
/// attacker's own life total stays where it was.
#[test]
fn yavimaya_wurm_tramples_five_past_the_creature_that_blocks_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 6])
        .hand(0, &[yavimaya_wurm()])
        .battlefield(1, &[llanowar_elves()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {4}{G}{G} is exactly the six Forests this board holds, so the cast is a
    // real payment out of a pool the lands actually filled.
    cast_from_hand(&mut engine, p0, yavimaya_wurm());
    pass_until(&mut engine, stack_is_empty);
    let wurm = on_battlefield(&engine, p0, yavimaya_wurm()).expect("the Wurm resolved");
    assert_eq!(pt(&engine, wurm), (6, 4), "the body the card prints");
    assert!(
        keywords(&engine, wurm).contains(KeywordSet::TRAMPLE),
        "and the one keyword it prints reaches the permanent"
    );

    // A whole turn, because a creature that entered this turn has summoning
    // sickness and is not offered as an attacker (CR 302.6).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let blocked = attack_and_collect_blocks(&mut engine, wurm, p1);
    assert_eq!(
        blocked.len(),
        1,
        "the 1/1 across the table may block the Wurm and is the only creature there"
    );
    let elf = blocked[0].blocker;
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, wurm)],
            },
        )
        .expect("the pairing the engine published");

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::Ending);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage is lethal to a printed 1/1 (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        15,
        "trample: the five damage past the blocker are the printed ability's whole point"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the excess went to the player being attacked, never back at the attacker's seat"
    );
    assert!(
        on_battlefield(&engine, p0, yavimaya_wurm()).is_some(),
        "a 6/4 survives the 1/1 it ran over"
    );
}
