//! `cards/creatures/mv_3/goblin_sky_raider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin Sky Raider is `{2}{R}` for a printed 1/2 whose whole text is flying,
/// so the keyword is only worth reading where it changes an answer: the combat
/// step. The board therefore carries a ground creature *and* a flier on the
/// other side, and the block declaration is read between them — the Llanowar
/// Elves are never paired with the Raider while the Baleful Strix is, which a
/// bare `keywords()` assertion could not tell apart from a pairing that
/// offered every blocker against every attacker. The attack waits a full turn
/// cycle, because a creature cast this turn is sick (CR 302.6), and the three
/// Mountains are read empty afterwards so the `{2}{R}` is a payment and not a
/// label.
#[test]
fn goblin_sky_raider_flies_over_a_ground_creature_and_is_met_only_by_a_flier() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[goblin_sky_raider()])
        .battlefield(1, &[llanowar_elves(), baleful_strix()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, goblin_sky_raider());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, goblin_sky_raider()).is_some() && stack_is_empty(e)
    });
    let raider = on_battlefield(&engine, p0, goblin_sky_raider()).expect("the Raider resolved");
    assert_eq!(pt(&engine, raider), (1, 2), "the 1/2 body the card prints");
    assert!(
        keywords(&engine, raider).contains(KeywordSet::FLYING),
        "and the one keyword it prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Mountains paid {{2}}{{R}} and left nothing floating"
    );

    // A creature cast this turn is sick (CR 302.6), so the attack happens on
    // p0's next turn, across p1's.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the attack declaration")
    };
    assert!(
        attackers.contains(&raider),
        "an untapped, unsick flier is offered as an attacker: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(raider, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the block declaration")
    };
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("their flier is out");
    let options_for = |blocker: ObjectId| -> Vec<ObjectId> {
        blockers
            .iter()
            .find(|option| option.blocker == blocker)
            .map_or_else(Vec::new, |option| option.attackers.clone())
    };
    assert!(
        !options_for(elves).contains(&raider),
        "a ground creature cannot block a flier: {:?}",
        options_for(elves)
    );
    assert!(
        options_for(strix).contains(&raider),
        "while a creature with flying is exactly what may: {:?}",
        options_for(strix)
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // Nothing blocked, so the one power the card prints is the damage it deals.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the unblocked Raider connected for its one power"
    );
}
