//! `cards/creatures/mv_2/moon_sprite.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Moon Sprite prints exactly two things: `{1}{G}` for a 1/1 Faerie and
/// "Flying". Both together are only worth something if the two mana really
/// disappear from the pool *and* flying changes the combat-step offer — so
/// across the table stands a ground creature that may not block the flyer,
/// and a second Moon Sprite as the one blocker that could. Without the
/// second, a missing offer could not be distinguished from a blocking step
/// that was never even made. The attack waits one turn cycle, because a
/// creature cast this turn has summoning sickness (CR 302.6).
#[test]
fn moon_sprite_flies_over_a_ground_blocker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[moon_sprite()])
        // The Elf is the control for "Flying", the second Sprite the reason
        // that the block question is even asked.
        .battlefield(1, &[llanowar_elves(), moon_sprite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Two Forests and nothing else on this side: no mana creature that
    // `tap_all_mana` would include (rule 11), so the two mana the spell
    // costs are readable from the pool.
    cast_from_hand(&mut engine, p0, moon_sprite());
    pass_until(&mut engine, stack_is_empty);
    let sprite = on_battlefield(&engine, p0, moon_sprite()).expect("the Sprite resolved");
    assert_eq!(
        pt(&engine, sprite),
        (1, 1),
        "{{1}}{{G}} buys the printed 1/1 body"
    );
    assert!(
        keywords(&engine, sprite).contains(KeywordSet::FLYING),
        "and the printed flying line reaches the permanent through the layers"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the two Forests paid for it"
    );

    // CR 302.6: cast this turn, so it can only attack after its own turn
    // has begun.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sprite, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the blocker declaration")
    };
    let ground = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is out");
    let flier = on_battlefield(&engine, p1, moon_sprite()).expect("their Sprite is out");
    assert!(
        blockers
            .iter()
            .all(|option| option.blocker != ground || !option.attackers.contains(&sprite)),
        "a ground creature may not block a flyer: {blockers:?}"
    );
    assert!(
        blockers
            .iter()
            .any(|option| option.blocker == flier && option.attackers.contains(&sprite)),
        "the only flyer there is offered exactly this pairing: {blockers:?}"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        19,
        "ein 1/1-Flieger, den niemand blocken konnte, trifft für eins"
    );
}
