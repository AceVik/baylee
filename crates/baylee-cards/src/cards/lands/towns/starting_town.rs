//! Starting Town — (no cost) — Land — Town
//! Oracle: This land enters tapped unless it's your first, second, or third turn of the game.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay 1 life: Add one mana of any color.
//! Set: FIN #289 — Final Fantasy | Scryfall ID: fc7d1912-7e27-49ef-bd98-375d975a42b0 | Oracle ID: d04e0975-f401-41b8-a9db-9bcf9cbbce66
// PARTIAL — both mana abilities are implemented; the enters-tapped clause is not.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STARTING_TOWN,
    oracle_id = "d04e0975-f401-41b8-a9db-9bcf9cbbce66",
    scryfall_id = "fc7d1912-7e27-49ef-bd98-375d975a42b0",
    faces = &[face!(
        name = "Starting Town",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::TOWN],
    ),],
    coverage = Coverage::Partial(
        "no EnterModifier variant asks how many turns of the game have passed, \
         so the \"enters tapped unless it's your first, second, or third turn\" \
         clause cannot be stated and the land always enters untapped",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!(TapSelf, PayLife(1)), &[Effect::mana_of_any_color()]),
    ],
);

// NOT SUPPORTED: "This land enters tapped unless it's your first, second, or third turn of the game." — every `EnterModifier::TappedUnless…` variant counts permanents, cards or seats; none counts turns of the game.
