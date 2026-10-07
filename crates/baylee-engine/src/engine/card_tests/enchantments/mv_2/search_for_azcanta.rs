//! `cards/enchantments/mv_2/search_for_azcanta.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Search for Azcanta` // `Azcanta, the Sunken Ruin` (`Coverage::Partial`):
/// "At the beginning of your upkeep, surveil 1. Then if you have seven or more cards in your
/// graveyard, you may transform `Search for Azcanta`. // `{{T}}`: Add `{{U}}`. `{{2}}{{U}}`, `{{T}}`: Look
/// at the top four cards of your library. You may reveal a noncreature, nonland card from among
/// them and put it into your hand. Put the rest on the bottom of your library in any order."
///
/// Under `Coverage::Partial`, the transform clause and the back face's card-selection ability
/// are omitted, while the upkeep surveil 1 trigger is implemented. The test advances to upkeep,
/// intercepts the surveil 1 arrangement (`ArrangePrompt::Surveil`), chooses to put the top
/// card into the graveyard, verifies the card arrives in the graveyard, and confirms that
/// `Search for Azcanta` remains on face 0.
#[test]
fn search_for_azcanta_surveils_at_upkeep_and_remains_on_face_zero() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(104, forest())
        .battlefield(0, &[search_for_azcanta(), island()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::Arrange {
                prompt: ArrangePrompt::Surveil,
                ..
            }
        )
    });

    let Pending::Arrange {
        cards,
        prompt,
        piles,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a surveil arrangement, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ArrangePrompt::Surveil);
    assert_eq!(piles, surveil_piles(1), "surveil 1 allows at most 1 card");
    assert_eq!(cards.len(), 1, "surveil 1 looks at the top card of library");

    let milled_card = cards[0];
    engine
        .apply(p0, look_answer(&cards, &[milled_card]))
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let azcanta = on_battlefield(&engine, p0, search_for_azcanta())
        .expect("Search for Azcanta on battlefield");
    assert_eq!(
        engine.state().object(azcanta).map(|o| o.face_index),
        Some(0),
        "Search for Azcanta remains on face 0"
    );
    assert!(
        types(&engine, azcanta).contains(TypeSet::ENCHANTMENT),
        "Search for Azcanta is an enchantment"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "the surveilled card was put into the graveyard"
    );
}
