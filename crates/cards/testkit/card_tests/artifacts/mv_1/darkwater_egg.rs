//! `cards/artifacts/mv_1/darkwater_egg.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Darkwater Egg — {1} artifact: "{2}, {T}, Sacrifice this artifact: Add
/// {U}{B}. Draw a card."
///
/// The three Islands pay the {1} and leave exactly the {2} the ability
/// charges, so the activation is a real payment out of the pool rather than a
/// label on a free ability — and with the mana already floating the offer is
/// read where the engine reads it (`can_afford` looks at the pool, not at
/// untapped lands). Afterwards the pool is one blue and one black and nothing
/// else: black has no other source on this board, so the {B} can only have
/// come off the Egg, and the two the ability made are exactly what the spent
/// {2} left behind.
///
/// Sacrificing the Egg is the second half of the price and is read as the
/// permanent leaving the battlefield for its owner's graveyard — the card is
/// gone, not merely tapped. The draw is the half no pool reading can see, so
/// it is asserted off the library and the hand together. The activated ability
/// uses the stack: costs happen first, mana and the draw only on resolution.
#[test]
fn darkwater_egg_trades_itself_for_blue_black_and_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[darkwater_egg()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, darkwater_egg());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, darkwater_egg()).is_some()
    });
    let egg = on_battlefield(&engine, p0, darkwater_egg()).expect("the Egg resolved");
    assert!(
        !is_tapped(&engine, egg),
        "an artifact enters untapped, so its {{T}} is still there to pay"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "three Islands paid the {{1}} and exactly the {{2}} the ability \
         charges is still floating"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(egg, 0)),
        "the one line the card prints, now that its {{2}} is in the pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, darkwater_egg(), 0);
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    assert!(in_graveyard(&engine, p0, darkwater_egg()).is_some());
    pass_until(&mut engine, stack_is_empty);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "{{B}} — and the only black source on this board is the Egg itself"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "and the {{U}} beside it"
    );
    assert_eq!(
        pool.total(),
        2,
        "the {{2}} was paid, so exactly the two mana the ability makes are left"
    );
    assert!(
        on_battlefield(&engine, p0, darkwater_egg()).is_none(),
        "sacrificing the Egg is the other half of its price"
    );
    assert!(
        in_graveyard(&engine, p0, darkwater_egg()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\""
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and the card reached the hand — an emptied library would satisfy the \
         count above without drawing anything"
    );
    assert!(
        stack_is_empty(&engine),
        "the activated ability has finished resolving"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat holds priority again, got {:?}",
        engine.pending()
    );
}
