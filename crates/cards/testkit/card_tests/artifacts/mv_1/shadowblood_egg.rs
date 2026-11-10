//! `cards/artifacts/mv_1/shadowblood_egg.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shadowblood Egg — {1} artifact: "{2}, {T}, Sacrifice this artifact:
/// Add {B}{R}. Draw a card."
///
/// One activation carries four printed things, and each is read off a
/// different place: the {2} out of a pool that only two Swamps paid into, the
/// tap and the sacrifice as a graveyard entry where a permanent stood, and the
/// draw as a library one shorter and a hand one longer. The offer read
/// *before* the mana is tapped is the control — the same board with an empty
/// pool must not list the ability, which is what says the {2} is a real price
/// and not a label on a free ability.
#[test]
fn shadowblood_egg_trades_itself_and_two_mana_for_black_red_and_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[shadowblood_egg(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let egg = on_battlefield(&engine, p0, shadowblood_egg()).expect("the Egg is on the table");

    // `LegalActions::abilities` is filtered through `can_afford`, and that
    // reads the pool rather than the untapped lands: no {2}, no offer.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == egg),
        "with an empty pool the {{2}} cannot be paid, so the Egg's one line is \
         not offered: {:?}",
        legal.abilities
    );

    // Now the mana. `tap_all_mana` would not press this ability anyway — its
    // whole price is not its own tap (#159) — but the Egg is named so that
    // the reading is about the two Swamps and nothing else.
    tap_all_mana_but(&mut engine, p0, Some(shadowblood_egg()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Swamps, two black, and nothing off the Egg"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(egg, 0)),
        "the {{2}} is floating, so the line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, shadowblood_egg(), 0);
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    assert!(in_graveyard(&engine, p0, shadowblood_egg()).is_some());
    pass_until(&mut engine, stack_is_empty);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "{{B}} arrived");
    assert_eq!(pool.available(ManaColor::Red), 1, "and so did {{R}}");
    assert_eq!(
        pool.total(),
        2,
        "the Swamps' mana was spent and two came back — black and red, and \
         nothing else on a board whose only other permanents are gone"
    );
    assert!(
        on_battlefield(&engine, p0, shadowblood_egg()).is_none(),
        "sacrificing the artifact is part of the cost, so it left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, shadowblood_egg()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\" — one off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and the card is in hand, not merely missing from the library"
    );
}
