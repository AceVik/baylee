//! Voldaren Estate — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast a Vampire spell.
//! Oracle: {5}, {T}: Create a Blood token. This ability costs {1} less to activate for each Vampire you control. (It's an artifact with "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.")
//! Set: LCC #369 — The Lost Caverns of Ixalan Commander | Scryfall ID: 1fcbc704-4fb3-49c3-9040-aa1681e86e2c | Oracle ID: fb0c0426-f1a6-4e52-9242-627786d3119a
// PARTIAL — {C}, and the {T} + pay 1 life line whose mana may only be spent on
// a Vampire spell; the {5}, {T} Blood ability is off the card (no rebate).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

// NOT SUPPORTED: "{5}, {T}: Create a Blood token. This ability costs {1} less to
// activate for each Vampire you control." — a cost reducer that counts a tribe is
// not sayable (`CostReduction` carries `NotStartingPlayer` alone), so the ability
// would charge the full {5} and the discount would be silently dropped.

card!(
    index = index::VOLDAREN_ESTATE,
    oracle_id = "fb0c0426-f1a6-4e52-9242-627786d3119a",
    scryfall_id = "1fcbc704-4fb3-49c3-9040-aa1681e86e2c",
    faces = &[face!(name = "Voldaren Estate", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "the \"{5}, {T}\" ability: \"This ability costs {1} less to activate for each Vampire you control\" is not expressible"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!(TapSelf, PayLife(1)),
            &[Effect::mana_of_any_color()
                .restricted(&Filter::HasSubtype(creature::VAMPIRE), SpendRider::None)]
        ),
    ],
);
