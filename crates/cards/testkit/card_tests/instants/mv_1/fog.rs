//! `cards/instants/mv_1/fog.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fog: "Prevent all combat damage that would be dealt this turn." An
/// unblocked 4-power attacker leaves its target untouched.
#[test]
fn fog_prevents_all_combat_damage_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[fog()])
        .battlefield(1, &[obsianus_golem()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let golem = on_battlefield(&engine, p1, obsianus_golem()).expect("seated");
    attack_and_collect_blocks(&mut engine, golem, p0);
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("p0 has nothing to block with");

    pass_until_priority(&mut engine, p0);
    let before = life_of(&engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, fog());
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain) && e.state().turn.active == p1
    });
    assert_eq!(
        life_of(&engine, p0),
        before,
        "an unblocked 4-power attacker leaves p0 untouched"
    );
}
