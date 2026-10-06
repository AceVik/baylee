//! `cards/creatures/artifacts/mv_4/juggernaut.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Martyrs of Korlis — "As long as this creature is untapped, all damage that
/// would be dealt to you by artifacts is dealt to this creature instead": a
/// replacement read as the damage would be dealt (CR 614.9), so the Copper
/// Tablet's ping at its controller is dealt to the untapped 1/6 instead. Once
/// the Martyrs has attacked and is tapped the "as long as" is false and
/// Juggernaut's combat damage reaches the player — one hop, never marked on
/// the creature on the way (CR 614.5).
#[allow(clippy::too_many_lines)] // one card read in both of its states on one board
#[test]
fn martyrs_of_korlis_takes_artifact_damage_while_untapped_and_stops_when_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[martyrs_of_korlis()])
        .battlefield(1, &[copper_tablet(), juggernaut()])
        .start();
    keep_mulligans(&mut engine);
    let martyrs = on_battlefield(&engine, p0, martyrs_of_korlis()).expect("seated");
    let jugg = on_battlefield(&engine, p1, juggernaut()).expect("seated");

    // p0's first upkeep: the Tablet's 1 is redirected to the untapped Martyrs.
    pass_until(&mut engine, |e| {
        at_rest(e, p0) && e.state().turn.step == Step::Upkeep
    });
    assert_eq!(engine.state().players[0].life, 20, "the player took none");
    assert_eq!(
        engine.state().object(martyrs).map(|o| o.damage),
        Some(1),
        "the untapped Martyrs took the artifact's point (CR 614.9)"
    );

    // p0 attacks with the Martyrs, tapping the redirector.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(martyrs, Defender::Player(p1))],
            },
        )
        .expect("the Martyrs may attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(is_tapped(&engine, martyrs));
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the Martyrs dealt its 1"
    );

    // p1's turn: Juggernaut attacks and the tapped Martyrs cannot take it.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let before = engine.journal().entries().len();
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(jugg, Defender::Player(p0))],
            },
        )
        .expect("the Juggernaut attacks each combat if able");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("the only creature is tapped and no blocker is required");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[0].life,
        15,
        "the 5 combat damage reached the player past a tapped Martyrs"
    );
    assert_eq!(
        damage_events(&engine, before),
        vec![(jugg, crate::event::DamageTarget::Player(p0), 5, true)],
        "one hop, from the Juggernaut to the player: the tapped Martyrs is \
         not in the damage's path"
    );
}
