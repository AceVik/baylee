//! `cards/creatures/mv_2/grimclaw_bats.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Grimclaw Bats` prints `KeywordSet::FLYING` and an activated ability `{{B}}, Pay 1 life`
/// granting +1/+1 until end of turn under `Coverage::Implemented`.
/// Activating the ability on an open board pays 1 life from the controller's life total (reducing it to 19)
/// and pumps the bat's `pt` from (1, 1) to (2, 2).
#[test]
fn grimclaw_bats_pays_life_to_pump() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1627, forest())
        .battlefield(0, &[grimclaw_bats(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bats = on_battlefield(&engine, p0, grimclaw_bats()).expect("bats are seated");
    assert_eq!(pt(&engine, bats), (1, 1));
    assert_eq!(engine.state().players[0].life, 20);
    assert!(keywords(&engine, bats).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, grimclaw_bats(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        19,
        "1 life paid as activation cost"
    );
    assert_eq!(pt(&engine, bats), (2, 2), "+1/+1 pump applied");
}
