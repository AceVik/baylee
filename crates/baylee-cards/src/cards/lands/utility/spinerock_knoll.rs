//! Spinerock Knoll — (no cost) — Land
//! Oracle: Hideaway 4 (When this land enters, look at the top four cards of your library, exile one face down, then put the rest on the bottom in a random order.)
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Oracle: {R}, {T}: You may play the exiled card without paying its mana cost if an opponent was dealt 7 or more damage this turn.
//! Set: DSC #300 — Duskmourn: House of Horror Commander | Scryfall ID: 11e4ba4b-62d1-43ed-82e8-cd45d7a62c65 | Oracle ID: 690c7f8e-fea2-4920-afa7-02ff120701a1
// PARTIAL — it enters tapped and taps for {R}; hideaway and the permission to
// play the exiled card need vocabulary the DSL does not have.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPINEROCK_KNOLL,
    oracle_id = "690c7f8e-fea2-4920-afa7-02ff120701a1",
    scryfall_id = "11e4ba4b-62d1-43ed-82e8-cd45d7a62c65",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Spinerock Knoll",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "Hideaway 4 (look at the top four, exile one face down, bottom the rest) and the {R}, {T} permission to play that exiled card have no DSL vocabulary"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        // NOT SUPPORTED: "Hideaway 4 (When this land enters, look at the top four cards of your library, exile one face down, then put the rest on the bottom in a random order.)" — no `Effect` looks at the top of a library and exiles one of those cards face down; `Effect::LookAtTopPick` sends the picked cards to hand, and nothing links a face-down exiled card to its source.
        // NOT SUPPORTED: "{R}, {T}: You may play the exiled card without paying its mana cost if an opponent was dealt 7 or more damage this turn." — the DSL has no permission to play a card exiled with the source (`Effect::WishToHand` only puts a card into hand), and no `Condition` for "an opponent was dealt 7 or more damage this turn".
    ],
);
