//! `cards/creatures/mv_4/rummaging_wizard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rummaging Wizard — {3}{U}, a 2/2 Human Wizard whose entire text is
/// "{2}{U}: Surveil 1". A surveil 1 is a question with two legal answers and
/// both are played here, because a test that only ever bottomed the card could
/// not tell the printed "may" from an unconditional mill, and one that only
/// ever kept it could not tell surveil from a scry. The top card is named
/// before the activation so the graveyard entry is provably *that* card, and
/// the pool afterwards says both {{2}}{{U}} payments came out of the ten
/// Islands rather than out of nothing.
#[test]
#[allow(clippy::too_many_lines)]
fn rummaging_wizard_surveils_one_each_way() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 10])
        .hand(0, &[rummaging_wizard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Ten Islands into the pool: {3}{U} for the creature and {2}{U} twice for
    // its ability. CR 500.5 empties a pool when a step ends, and the whole
    // scenario lives inside this one main phase.
    cast_from_hand(&mut engine, p0, rummaging_wizard());
    pass_until(&mut engine, stack_is_empty);
    let wizard = on_battlefield(&engine, p0, rummaging_wizard()).expect("the Wizard resolved");
    assert_eq!(pt(&engine, wizard), (2, 2), "the printed body");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "ten Islands less the {{3}}{{U}} the cast cost"
    );

    let library_before = library_size(&engine, p0);
    let listed = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *listed.last().expect("p0 has a library");
    let second = listed[listed.len() - 2];

    // First activation: the top card goes to the graveyard.
    activate(&mut engine, p0, rummaging_wizard(), 0);
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
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the Wizard's controller does the looking");
    assert_eq!(
        prompt,
        ArrangePrompt::Surveil,
        "the variant is what tells a client this card goes to a graveyard and \
         not to the bottom of a library"
    );
    assert_eq!(
        piles,
        surveil_piles(1),
        "either the card is put into the graveyard or it is not"
    );
    assert_eq!(cards, vec![top], "the top card, and only it");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{2}}{{U}} came out of the pool"
    );
    assert!(
        !is_tapped(&engine, wizard),
        "the price is mana and no {{T}}, so the Wizard is still standing"
    );

    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("a card the surveil put on the menu is a legal answer");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&top),
        "the chosen card lies in the graveyard (CR 701.25a)"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and it left the library, so a surveil that only looked would not \
         satisfy this"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(second),
        "the card that was second is now on top"
    );

    // Second activation: the new top card is left where it is, which is the
    // other half of the printed "may".
    activate(&mut engine, p0, rummaging_wizard(), 0);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::Arrange {
                prompt: ArrangePrompt::Surveil,
                ..
            }
        )
    });
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(cards, vec![second], "the new top card, and only it");
    engine
        .apply(p0, look_answer(&cards, &[]))
        .expect("declining to put it into the graveyard is a legal answer");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "both activations paid their {{2}}{{U}}"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the second card was left on top, so the library did not shrink again"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "and nothing new went to the graveyard"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(second),
        "the card that was looked at a second time is still the top of the library"
    );
    assert!(
        on_battlefield(&engine, p0, rummaging_wizard()).is_some(),
        "the Wizard paid nothing but mana and is still standing after both activations"
    );
}
