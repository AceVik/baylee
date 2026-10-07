//! `cards/creatures/mv_6/dread_reaper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dread Reaper is a 6/5 flying Horror under `Coverage::Implemented` costing {3}{B}{B}{B}.
/// When it enters the battlefield, an enters-the-battlefield trigger is put on the stack causing its controller to lose 5 life.
/// The trigger resolves, reducing only its controller's life total from 20 to 15 while leaving the opponent untouched.
/// The resulting creature permanent projects flying and printed 6/5 power and toughness.
#[test]
fn dread_reaper_enters_causes_five_life_loss_and_has_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[dread_reaper()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, dread_reaper()).expect("Dread Reaper is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{3}}{{B}}{{B}}{{B}}, so casting is not offered"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Swamps produce six black mana"
    );
    cast_with_floating(&mut engine, p0, dread_reaper());

    pass_until(&mut engine, stack_is_empty);

    let reaper =
        on_battlefield(&engine, p0, dread_reaper()).expect("Dread Reaper entered battlefield");
    assert_eq!(
        pt(&engine, reaper),
        (6, 5),
        "printed power and toughness is 6/5"
    );
    assert!(
        keywords(&engine, reaper).contains(KeywordSet::FLYING),
        "Dread Reaper has flying"
    );
    assert_eq!(
        engine.state().players[0].life,
        15,
        "controller lost 5 life from ETB trigger"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "opponent life total remains unchanged"
    );
}
