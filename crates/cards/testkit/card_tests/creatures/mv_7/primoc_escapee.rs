//! `cards/creatures/mv_7/primoc_escapee.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "28a1e053-eb14-4fbf-afd1-bfb6117a839a"

/// Primoc Escapee prints two lines — a {6}{U} 4/4 Bird Beast with flying, and
/// cycling {2} — and the second one is an ability that lives in the *hand* and
/// is paid for with the card itself, so nothing but playing it can say whether
/// the engine offers it there. The scenario plays both halves: nine Islands
/// cast the creature in one main phase, and the two mana left floating beside
/// it pay the cycling, which has to move the discarded card into the graveyard
/// and leave a fresh card in the hand. The price is also read the way the
/// engine reads it (`can_afford` looks at the pool and not at the untapped
/// lands), so the line is asserted absent before the Islands are tapped and
/// present afterwards.
#[test]
fn primoc_escapee_flies_and_cycles_itself_out_of_hand_for_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 9])
        .hand(0, &[primoc_escapee(), primoc_escapee()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Whether the hand ability is on offer, read where the engine decides it:
    // the list is filtered through `can_afford`, which reads the pool rather
    // than the nine untapped Islands standing beside it.
    let cycling_offered = |engine: &Engine<RegistryLookup>| -> bool {
        let Pending::Priority { legal, .. } = engine.pending() else {
            return false;
        };
        legal.abilities.iter().any(|(id, _)| {
            engine.state().object(*id).is_some_and(|o| {
                o.zone == Zone::Hand && o.card.is_some_and(|c| c.index == primoc_escapee())
            })
        })
    };
    assert!(
        !cycling_offered(&engine),
        "an empty pool pays no {{2}}, so the cycling line is not offered at all"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        9,
        "nine Islands and nothing else on the board, so nine blue"
    );

    // {6}{U} for the creature, leaving exactly the {2} the cycling charges in
    // the pool — CR 500.5 keeps a pool across a cast, and this all happens
    // inside one main phase.
    cast_with_floating(&mut engine, p0, primoc_escapee());
    pass_until(&mut engine, stack_is_empty);

    let bird = on_battlefield(&engine, p0, primoc_escapee()).expect("the Escapee resolved");
    assert_eq!(pt(&engine, bird), (4, 4), "the printed body");
    assert!(
        keywords(&engine, bird).contains(KeywordSet::FLYING),
        "the front half of the card reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "seven mana for the creature out of the nine the Islands made"
    );
    assert!(
        cycling_offered(&engine),
        "with the {{2}} floating, the copy still in hand offers its cycling"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Ability 0 is the printed "Cycling {2}" — the card's only ability, and one
    // the offer names on an object that is still sitting in the hand.
    activate(&mut engine, p0, primoc_escapee(), 0);

    assert!(
        in_graveyard(&engine, p0, primoc_escapee()).is_some(),
        "\"Discard this card\" is half the price and is paid on announcement \
         (CR 601.2h), so the cycled card is in its owner's graveyard already"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the other half was the {{2}} that was floating"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the cycling is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card discarded and one drawn, so the hand is the size it was"
    );
    assert!(
        on_battlefield(&engine, p0, primoc_escapee()).is_some(),
        "the creature that was cast is not the card that was cycled"
    );
    assert!(
        in_hand(&engine, p0, primoc_escapee()).is_none(),
        "and both copies have left the hand: one to the battlefield, one to the graveyard"
    );
}
