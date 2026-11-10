//! `cards/creatures/mv_1/lantern_kami.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lantern Kami prints one word — flying — on a `{W}` 1/1 Spirit, so the
/// whole card is that it arrives for one white mana and that the word does
/// something. The copy cast from hand proves the cost and the body; the copy
/// that has stood on the battlefield since the turn began makes the attack,
/// because CR 302.6 keeps a creature that entered this turn out of combat
/// entirely and an evasion claim tested on such a body would never reach the
/// rule. The two blockers across the table draw the line: their own Lantern
/// Kami may be paired with it, their Llanowar Elves may not.
#[test]
fn lantern_kami_flies_over_a_ground_creature_and_is_blocked_only_from_the_air() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[plains(), lantern_kami()])
        .hand(0, &[lantern_kami()])
        // A flier and a ground creature: one may block what flies, one may not.
        .battlefield(1, &[lantern_kami(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let resident = on_battlefield(&engine, p0, lantern_kami()).expect("the Kami is on the table");
    cast_from_hand(&mut engine, p0, lantern_kami());
    pass_until(&mut engine, stack_is_empty);
    let cast = all_on_battlefield(&engine, p0, lantern_kami())
        .into_iter()
        .find(|id| *id != resident)
        .expect("the Kami cast this turn resolved");
    assert_eq!(
        pt(&engine, cast),
        (1, 1),
        "a printed 1/1 for one white mana"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the Plains and the {{W}} it made are both spent"
    );
    assert!(
        keywords(&engine, cast).contains(KeywordSet::FLYING),
        "the whole of its printed text, read after the layers"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on the attack declaration")
    };
    assert_eq!(player, p0, "the Kami's controller declares");
    assert!(
        attackers.contains(&resident),
        "the Kami that has been here since the turn began may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(resident, Defender::Player(p1))],
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
        unreachable!("pass_until stops on the block declaration")
    };
    assert_eq!(player, p1, "the defending seat is the one asked");
    let their_kami = on_battlefield(&engine, p1, lantern_kami()).expect("their Kami is out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let may_block = |blocker: ObjectId| -> Vec<ObjectId> {
        blockers
            .iter()
            .find(|option| option.blocker == blocker)
            .map(|option| option.attackers.clone())
            .unwrap_or_default()
    };
    assert!(
        may_block(their_kami).contains(&resident),
        "flying is what lets one flier block another: {:?}",
        may_block(their_kami)
    );
    assert!(
        !may_block(their_elf).contains(&resident),
        "a creature with no flying is never paired with it: {:?}",
        may_block(their_elf)
    );

    let life = engine.state().players[1].life;
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        life - 1,
        "the unblocked flier dealt the one damage it prints"
    );
}
