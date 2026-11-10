//! `cards/creatures/mv_5/sandbar_serpent.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sandbar Serpent is a `{4}{U}` 3/4 Serpent whose entire printed rules text
/// is "Cycling `{2}`" — an activated ability that lives in the *hand*
/// (`ActivationZone::Hand`) and whose whole price is two mana and the card
/// itself. Casting it would say nothing about that line, so the scenario
/// cycles: two Forests pay the `{2}`, the ability is read out of
/// `LegalActions::abilities` while the mana is really floating (the list is
/// filtered through `can_afford`, which reads the pool and not the untapped
/// lands), and the two halves of the swap are read in different zones — the
/// discard as a graveyard entry, the draw on the library and the hand
/// together, because a card that merely vanished from the hand would satisfy
/// either reading alone.
#[test]
fn sandbar_serpent_cycles_itself_away_for_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[sandbar_serpent()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let serpent = in_hand(&engine, p0, sandbar_serpent()).expect("the Serpent is in hand");
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let yard_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    // Mana before the claim: the `{2}` is read off the pool, so nothing is
    // asserted about the offer until the two Forests are actually tapped.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests, two green — exactly the printed cycling cost, and the \
         only source on this board"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(serpent, 0)),
        "cycling is a hand-zone ability, so the card in hand is the offered \
         source: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sandbar_serpent(), 0);

    // CR 601.2h: the discard is a cost and is paid on announcement; the draw
    // is the effect and waits on the stack.
    assert!(
        in_graveyard(&engine, p0, sandbar_serpent()).is_some(),
        "\"Discard this card\" is half the price, so it is already in its \
         owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, sandbar_serpent()).is_none(),
        "and the card is nowhere on the battlefield: cycling is not a cast"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability uses the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the Serpent left the hand as half the price and the drawn card \
         replaced it — a discard that never happened would leave this one \
         long, and a draw that never happened one short"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        yard_before + 1,
        "and exactly one card arrived in the graveyard, which is the Serpent \
         rather than something the draw put there"
    );
    assert!(
        on_battlefield(&engine, p0, sandbar_serpent()).is_none(),
        "cycling never puts the creature onto the battlefield"
    );
}
