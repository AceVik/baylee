//! `cards/artifacts/mv_1/soul_net.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Soul Net: "Whenever a creature dies, you may pay {1}. If you do, you
/// gain 1 life." Killed the harness way (`bury`, not `kill`: that helper's
/// own `pass_until(stack_is_empty)` would run straight past the tax
/// question this test needs to answer).
#[test]
fn soul_net_offers_to_pay_and_gain_life_when_a_creature_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[soul_net(), forest(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    bury(&mut engine, &[elf]);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!(
            "expected priority after the kill, got {:?}",
            engine.pending()
        )
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();

    pays_the_tax_and_gains_a_life(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, quiet_creature()).is_some());
}
