//! `cards/enchantments/mv_2/circle_of_protection_blue.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Circle of Protection: Blue: "{1}: The next time a blue source of your
/// choice would deal damage to you this turn, prevent that damage." Baleful
/// Strix, a blue attacker, is the only blue source on the board, and its
/// combat damage is fully prevented once chosen.
#[test]
fn circle_of_protection_blue_prevents_damage_from_a_chosen_blue_attacker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let cop = circle_of_protection_blue();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cop, forest()])
        .battlefield(1, &[baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("their Strix");
    reach_their_main_phase(&mut engine, p1);

    let blockers = attack_and_collect_blocks(&mut engine, strix, p0);
    assert!(blockers.is_empty(), "p0 has nothing to block with");
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, cop, 0);
    let selected = baylee_core::ids::DamageSourceRef {
        object: strix,
        version: engine.state().object(strix).expect("source exists").version,
    };
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        options,
        choice,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0);
    assert_eq!(options, vec![selected], "the only blue source on the board");
    engine
        .apply(
            p0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: selected,
            },
        )
        .expect("off the list");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the Strix's combat damage was prevented"
    );
    assert_eq!(engine.state().players[1].life, 20);
}
