//! `cards/artifacts/mv_3/forcefield.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Forcefield: "{1}: The next time an unblocked creature of your choice
/// would deal combat damage to you this turn, prevent all but 1 of that
/// damage." Before blockers are declared no attacker is unblocked (CR
/// 509.1h), so the same activation chooses nothing and leaves no shield;
/// once the Hill Giant is unblocked, it deals 1 of its 3.
#[test]
fn forcefield_lets_one_of_an_unblocked_creatures_damage_through() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(43, forest())
        .battlefield(0, &[hill_giant()])
        .battlefield(1, &[forcefield(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("deployed");
    reach_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1))],
            },
        )
        .expect("the Giant attacks");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    assert_eq!(
        engine.state().turn.step,
        crate::turn::Step::DeclareAttackers
    );
    let spare = all_on_battlefield(&engine, p1, plains())[1];
    tap_mana_except(&mut engine, p1, spare);
    activate(&mut engine, p1, forcefield(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine.state().shields.is_empty(),
        "an attacker is not yet unblocked, so there was nothing to choose"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, forcefield(), 0);
    let selected = baylee_core::ids::DamageSourceRef {
        object: giant,
        version: engine.state().object(giant).expect("source exists").version,
    };
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        options, choice, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(options, vec![selected], "the unblocked attacker");
    engine
        .apply(
            p1,
            PlayerAction::ChooseDamageSource {
                choice,
                source: selected,
            },
        )
        .expect("off the list");
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    assert_eq!(engine.state().players[1].life, 19, "1 of the Giant's 3");
    assert!(engine.state().shields.is_empty(), "and the shield is spent");
}
