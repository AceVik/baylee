//! Aladdin's Lamp — {10} — Artifact
//! Oracle: {X}, {T}: The next time you would draw a card this turn, instead look at the top X cards of your library, put all but one of them on the bottom of your library in a random order, then draw a card. X can't be 0.
//! Set: 4ED #291 — Fourth Edition | Scryfall ID: 42e7cf40-c136-4fcb-a947-558b713b39f6 | Oracle ID: 2aa2f96b-5784-4767-b9ea-b8d9222cb1de
// IMPLEMENTED — {X}, {T}: the next draw this turn becomes a look at the top X
// cards, keep one, the rest on the bottom at random, then draw; X is at least 1.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALADDIN_S_LAMP,
    oracle_id = "2aa2f96b-5784-4767-b9ea-b8d9222cb1de",
    scryfall_id = "42e7cf40-c136-4fcb-a947-558b713b39f6",
    faces = &[face!(
        name = "Aladdin's Lamp",
        mana_cost = mana!("{10}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{X}", TapSelf),
        &[Effect::NextDrawLooksAtTop { count: Amount::X }],
        x_value = ActivationX::AtLeast(1),
    )],
);
