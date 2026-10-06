//! `cards/creatures/mv_5/sengir_vampire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sengir Vampire — {3}{B}{B} 4/4 Vampire. Attacking with it, the opponent's
/// non-flying, non-reach Llanowar Elves cannot legally be assigned to
/// block — while a flying Wall of Swords beside it still may, which is
/// what says the menu reads flying and not "nothing may block it" — and
/// the unblocked flier's 4 damage connects.
#[test]
fn sengir_vampire_cannot_be_blocked_by_a_creature_without_flying_or_reach() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sengir_vampire()])
        .battlefield(1, &[llanowar_elves(), wall_of_swords()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vamp = on_battlefield(&engine, p0, sengir_vampire()).expect("seated");
    assert_eq!(pt(&engine, vamp), (4, 4), "the body the card prints");
    assert!(
        keywords(&engine, vamp).contains(KeywordSet::FLYING),
        "\"Flying\" is the printed line"
    );
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");
    let flying_wall = on_battlefield(&engine, p1, wall_of_swords()).expect("seated");

    let blocks = attack_and_collect_blocks(&mut engine, vamp, p1);
    assert!(
        !blocks
            .iter()
            .any(|b| b.blocker == elf && b.attackers.contains(&vamp)),
        "a non-flying, non-reach creature cannot legally block a flier: {blocks:?}"
    );
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == flying_wall && b.attackers.contains(&vamp)),
        "a flier beside it is still offered as a legal blocker: {blocks:?}"
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declining every legal block still stands");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[1].life,
        16,
        "unblocked, all 4 of the flier's damage connects"
    );
}
