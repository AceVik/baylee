//! `cards/creatures/mv_4/lightning_hounds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lightning Hounds — {2}{R}{R} — 3/2 Dog with first strike.
///
/// First strike is only visible in what a blocked exchange leaves behind, so the
/// defender gets a vanilla 3/3: the pool's quietest creature with two +1/+1
/// counters on it, a body whose three power would kill the Hounds' two toughness
/// if both struck at once. The Hounds deals its three damage in the first-strike
/// damage step (CR 510.4), so the 3/3 is dead before the regular damage step ever
/// arrives and the Hounds is still on the battlefield, which a simultaneous
/// exchange could not leave behind.
#[test]
fn lightning_hounds_kills_a_three_three_in_the_first_strike_damage_step() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lightning_hounds()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    // The Elf's whole printed text is one mana ability, so the only thing on
    // this board that can decide the exchange is the body the counters give it.
    let blocker = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is out");
    let state = engine
        .dev_state_mut(p1)
        .expect("the harness may set boards up");
    crate::replacement::put_counters(state, blocker, CounterKind::P1P1, 2);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    assert_eq!(
        counters_on(&engine, blocker, CounterKind::P1P1),
        2,
        "two +1/+1 counters, read back off the permanent"
    );
    assert_eq!(
        pt(&engine, blocker),
        (3, 3),
        "and the projection turns them into the 3/3 that would trade with the Hounds"
    );

    let hounds = on_battlefield(&engine, p0, lightning_hounds()).expect("the Hounds are out");
    assert_eq!(pt(&engine, hounds), (3, 2), "the body the card prints");
    assert!(
        keywords(&engine, hounds).contains(KeywordSet::FIRST_STRIKE),
        "the printed first strike reaches the permanent"
    );

    let blocks = attack_and_collect_blocks(&mut engine, hounds, p1);
    assert!(
        blocks
            .iter()
            .any(|o| o.blocker == blocker && o.attackers.contains(&hounds)),
        "the 3/3 is offered as a blocker for the only attacker: {blocks:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, hounds)],
            },
        )
        .expect("the blocker came out of the list that offered it");

    // Not `stack_is_empty`: the stack is empty the instant blockers are
    // declared, and what this test needs is the far side of both damage steps.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending) && e.state().turn.active == p0
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "three first-strike damage is lethal to a 3/3 (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p0, lightning_hounds()).is_some(),
        "and the 3/3 was gone before the regular damage step, so nothing struck \
         back at the 3/2 — an exchange both creatures survive is not one that \
         would have taken the Hounds with it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that blocked and never to the player behind it"
    );
}
