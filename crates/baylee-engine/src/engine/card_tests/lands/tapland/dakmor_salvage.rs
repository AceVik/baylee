//! `cards/lands/tapland/dakmor_salvage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Three shelf lands with one printed sentence between them — "this land
/// enters tapped" — and all of them a mana ability to go with it. They are
/// played together because the assertion that matters is the pair: a land
/// that entered untapped would still make its mana, and a colour is only a
/// claim beside a total.
///
/// Cryptic Spires also enters tapped and produces exactly one of the two
/// colors recorded for this physical copy while constructing its deck.
#[test]
fn three_shelf_taplands_enter_tapped_and_make_the_colour_they_print() {
    let p0 = PlayerId::new(0);

    for (card, colour) in [
        (dakmor_salvage(), ManaColor::Black),
        (spinerock_knoll(), ManaColor::Red),
    ] {
        let mut engine = Duel::new(SEED, forest()).hand(0, &[card]).start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
        let land = play_land(&mut engine, p0, card);
        assert!(
            entered_tapped(&engine, land),
            "\"This land enters tapped\" is unconditional on this card"
        );

        cross_into_the_next_own_main(&mut engine, p0);
        activate(&mut engine, p0, card, 0);
        assert_eq!(
            engine.state().players[0].mana_pool.available(colour),
            1,
            "the one colour this land's ability prints"
        );
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            1,
            "and nothing beside it"
        );
    }

    let mut spires = Duel::new(SEED, forest())
        .hand(0, &[cryptic_spires()])
        .start();
    keep_mulligans(&mut spires);
    assert!(walk_to_own_main(&mut spires, p0), "p0 reaches its own main");
    let hand = in_hand(&spires, p0, cryptic_spires()).unwrap();
    spires.state.object_mut(hand).unwrap().deck_choices = baylee_core::card_metadata::DeckChoices {
        circled_colors: Some(baylee_core::color::ColorPair::new(
            baylee_core::color::Color::White,
            baylee_core::color::Color::Black,
        )),
    };
    let land = play_land(&mut spires, p0, cryptic_spires());
    assert!(entered_tapped(&spires, land), "it enters tapped too");

    cross_into_the_next_own_main(&mut spires, p0);
    assert!(
        !is_tapped(&spires, land),
        "and untaps like any other permanent"
    );
    let Pending::Priority { legal, .. } = spires.pending().clone() else {
        panic!("expected priority, got {:?}", spires.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "the circled-color mana action is offered"
    );
    activate(&mut spires, p0, cryptic_spires(), 0);
    let Pending::ChooseColor { options, .. } = spires.pending() else {
        panic!("circled colors are asked")
    };
    assert_eq!(options, &[ManaColor::White, ManaColor::Black]);
    spires
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    assert_eq!(
        spires.state.players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
    assert_eq!(spires.state.players[0].mana_pool.total(), 1);
    assert!(is_tapped(&spires, land));
}
