//! `cards/creatures/mv_6/force_of_nature.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Force of Nature — Trample; "At the beginning of your upkeep, this
/// creature deals 8 damage to you unless you pay `{G}{G}{G}{G}`." Declined,
/// its controller takes the 8.
#[test]
fn force_of_nature_deals_8_to_its_controller_unless_gggg_is_paid() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[force_of_nature()])
        .start();
    keep_mulligans(&mut engine);
    let treant = on_battlefield(&engine, p0, force_of_nature()).expect("seated");
    assert_eq!(pt(&engine, treant), (8, 8));
    assert!(keywords(&engine, treant).contains(KeywordSet::TRAMPLE));

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 12, "8 damage, left unpaid");
    assert!(on_battlefield(&engine, p0, force_of_nature()).is_some());
}

/// Force of Nature, the other branch: paying the `{G}{G}{G}{G}` spares its
/// controller the damage. Life alone at 20 would also hold for a cost of
/// `{G}` or free, so the pool is checked too: exactly four Forests stand
/// here, and the payment window leaves the pool empty — not merely
/// nonzero — showing the full `{G}{G}{G}{G}` was what left it.
#[test]
fn force_of_nature_paying_gggg_spares_its_controller_the_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[force_of_nature(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let made = tap_all_mana(&mut engine, p0);
    assert_eq!(made, 4, "all four Forests, and no more, stand here");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four green floating, matching the printed cost exactly"
    );
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the payment window spent every last one of the four: not {{G}}, not free"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "paid the {{G}}{{G}}{{G}}{{G}}"
    );
}
