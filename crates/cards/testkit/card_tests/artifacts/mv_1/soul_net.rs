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

/// Soul Net: "you may pay {1}" is a choice. Declined, a dying creature gains
/// no life and the mana floating in the pool is not spent.
#[test]
fn soul_net_declined_gains_nothing_and_spends_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[soul_net(), forest(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    bury(&mut engine, &[elf]);
    let life = life_of(&engine, p0);
    let floating = engine.state().players[0].mana_pool.total();
    assert!(floating >= 1, "something is floating to pay with");
    assert_eq!(the_tax_is_asked_of(&mut engine), p0);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(life_of(&engine, p0), life, "declined: no life");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        floating,
        "declined: the {{1}} is not spent"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), life);
}

/// Soul Net: "Whenever *a* creature dies": an opponent's Elf dying asks the
/// Net's controller, who pays and gains the life.
#[test]
fn soul_net_pays_its_controller_when_an_opponents_creature_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[soul_net(), forest()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf");
    let before = life_of(&engine, p0);
    bury(&mut engine, &[elf]);
    assert_eq!(
        the_tax_is_asked_of(&mut engine),
        p0,
        "asked of the Net's owner"
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(life_of(&engine, p0), before + 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, quiet_creature()).is_some());
}

/// Soul Net: only a *creature* dying is asked about. A non-creature
/// permanent (Sol Ring) dying raises no question and no life.
#[test]
fn soul_net_ignores_a_noncreature_permanent_dying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[soul_net(), forest()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact");
    let before = life_of(&engine, p0);
    bury(&mut engine, &[ring]);
    assert!(in_graveyard(&engine, p1, quiet_artifact()).is_some());
    // Any tax question would panic `pass_until` on the way round.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(life_of(&engine, p0), before, "no creature died");
}
