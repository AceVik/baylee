//! Urban Retreat — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}, {W}, or {U}.
//! Oracle: {2}, Return a tapped creature you control to its owner's hand: Put this card from your hand onto the battlefield. Activate only as a sorcery.
//! Set: SPM #187 — Marvel's Spider-Man | Scryfall ID: 2581f320-8238-413d-ab04-d5535da55630 | Oracle ID: 18290c4b-cff6-4ee5-a487-fe0d5706d3de
// IMPLEMENTED — enters tapped, and {T} adds {G}, {W} or {U}. The third
// ability is dropped: no `Effect` moves the source card from its owner's
// hand onto the battlefield (see the `// NOT SUPPORTED:` line), so the
// card is `Coverage::Partial`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::URBAN_RETREAT,
    oracle_id = "18290c4b-cff6-4ee5-a487-fe0d5706d3de",
    scryfall_id = "2581f320-8238-413d-ab04-d5535da55630",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    faces = &[face!(
        name = "Urban Retreat",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "the third ability's effect — put this card from your hand onto the battlefield — has no Effect variant",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[
            ManaColor::Green,
            ManaColor::White,
            ManaColor::Blue,
        ])]),
        // NOT SUPPORTED: "{2}, Return a tapped creature you control to its owner's
        // hand: Put this card from your hand onto the battlefield. Activate only as
        // a sorcery." — the cost is sayable (CostPart::ReturnToHand on a
        // hand-zone activation), but no `Effect` puts the source card from a hand
        // onto the battlefield, so the ability comes off the card.
    ],
);
