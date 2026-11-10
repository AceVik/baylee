//! `cards/creatures/mv_6/aang_and_katara.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aang and Katara: "Whenever Aang and Katara enter or attack, create X 1/1
/// white Ally creature tokens, where X is the number of tapped artifacts
/// and/or creatures you control." Attacking taps Aang himself, so with Sol
/// Ring and an Elf tapped for mana X is three.
#[test]
fn aang_and_katara_attacking_makes_a_token_per_tapped_artifact_or_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4503, island())
        .battlefield(0, &[aang_and_katara(), sol_ring(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let aang = on_battlefield(&engine, p0, aang_and_katara()).expect("Aang and Katara");
    tap_all_mana(&mut engine, p0);
    assert!(tokens_of(&engine, p0).is_empty());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(aang, Defender::Player(p1))],
            },
        )
        .expect("Aang attacks");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        tokens_of(&engine, p0).len(),
        3,
        "Sol Ring, the Elf and Aang himself are tapped"
    );
}
