//! Port Town — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Plains or Island card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {W} or {U}.
//! Set: MSC #256 — Marvel Super Heroes Commander | Scryfall ID: 3f962cab-c058-41f5-b7e1-5b063f374eb0 | Oracle ID: 458d2b12-f578-4392-98d3-c3bc83f316c4
// IMPLEMENTED — the mana ability, which chooses {W} or {U} on resolution.
// The entry clause is not: no EnterModifier says "reveal a card from your
// hand, or this enters tapped".

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PORT_TOWN,
    oracle_id = "458d2b12-f578-4392-98d3-c3bc83f316c4",
    scryfall_id = "3f962cab-c058-41f5-b7e1-5b063f374eb0",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(name = "Port Town", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "As this land enters, you may reveal a Plains or Island card from your hand; \
         if you don't, it enters tapped — there is no EnterModifier for a reveal \
         from hand, so this land always enters untapped"
    ),
    // NOT SUPPORTED: As this land enters, you may reveal a Plains or Island
    // card from your hand. If you don't, this land enters tapped.
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::White,
        ManaColor::Blue,
    ])])],
);
