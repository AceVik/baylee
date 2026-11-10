//! `cards/artifacts/mv_1/mossfire_egg.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mossfire Egg is `{1}` for one line: "`{2}`, `{T}`, Sacrifice this
/// artifact: Add `{R}{G}`. Draw a card." Moving a card from the library
/// makes this an ordinary activated ability under the current rules, so the
/// sacrifice and payment precede the stack's resolution; mana and draw follow.
/// Three Plains pay the `{1}` and the `{2}` and are spent down to nothing, so
/// exactly one red and one green are left: neither is a colour any permanent
/// on this board could have produced. The Egg is read in its owner's
/// graveyard rather than merely gone from the battlefield, because the
/// sacrifice is a cost and a cost that silently never happened looks exactly
/// like one that did.
#[test]
fn mossfire_egg_sacrifices_itself_then_resolves_for_red_green_and_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7331, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[mossfire_egg()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1} off the Plains, and the {2} the ability charges stays in the pool
    // beside it: CR 500.5 empties a pool at the end of a step, and the whole
    // scenario plays inside this one main phase.
    cast_from_hand(&mut engine, p0, mossfire_egg());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let egg = on_battlefield(&engine, p0, mossfire_egg()).expect("the Egg resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "three Plains paid the {{1}} and left exactly the {{2}}"
    );
    assert!(!is_tapped(&engine, egg), "and it enters untapped and ready");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(egg, 0)),
        "with the {{2}} already floating, the one line the Egg prints is \
         offered: {:?}",
        legal.abilities
    );
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);

    activate(&mut engine, p0, mossfire_egg(), 0);
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    assert!(in_graveyard(&engine, p0, mossfire_egg()).is_some());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        stack_is_empty(&engine),
        "the activated ability has finished resolving"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the activating seat holds priority again, got {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "Add {{R}}");
    assert_eq!(pool.available(ManaColor::Green), 1, "and Add {{G}}");
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "the Plains' white went into the {{2}}, so neither colour left in the \
         pool has a source still standing on this board"
    );
    assert_eq!(pool.total(), 2, "two mana, one of each, and nothing else");
    assert!(
        on_battlefield(&engine, p0, mossfire_egg()).is_none(),
        "the sacrifice is a cost, so the Egg is gone the moment it is paid"
    );
    assert!(
        in_graveyard(&engine, p0, mossfire_egg()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "\"Draw a card\" — the half of the line no plain mana source prints"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card off the top of the library, so the hand grew by a draw"
    );
}
