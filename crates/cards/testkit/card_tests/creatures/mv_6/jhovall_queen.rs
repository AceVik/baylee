//! `cards/creatures/mv_6/jhovall_queen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Jhovall Queen` is a 4/7 creature costing `{4}{W}{W}` under `Coverage::Implemented`.
/// It prints the vigilance keyword.
/// When declaring attackers in combat, a creature with vigilance does not tap to attack,
/// remaining untapped after attacking.
#[test]
fn jhovall_queen_attacks_without_tapping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[jhovall_queen()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let queen =
        on_battlefield(&engine, p0, jhovall_queen()).expect("Jhovall Queen is on the battlefield");
    assert_eq!(pt(&engine, queen), (4, 7), "body is 4/7");
    assert!(
        keywords(&engine, queen).contains(KeywordSet::VIGILANCE),
        "has vigilance"
    );
    assert!(!is_tapped(&engine, queen), "starts untapped");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(queen, Defender::Player(p1))],
            },
        )
        .unwrap();

    assert!(
        !is_tapped(&engine, queen),
        "creature with vigilance does not tap when declared as an attacker"
    );
}
