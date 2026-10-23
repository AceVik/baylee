//! Limited Resources — {W} — Enchantment
//! Oracle: When this enchantment enters, each player chooses five lands they control and sacrifices the rest.
//! Oracle: Players can't play lands as long as ten or more lands are on the battlefield.
//! Set: EXO #10 — Exodus | Scryfall ID: 20ae3609-a3cc-486c-94f6-b8f647adfb47 | Oracle ID: 5b287dcc-f853-4df1-8ddf-5b8b2adc2873

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIMITED_RESOURCES,
    oracle_id = "5b287dcc-f853-4df1-8ddf-5b8b2adc2873",
    scryfall_id = "20ae3609-a3cc-486c-94f6-b8f647adfb47",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "The enters-the-battlefield land sacrifice trigger and the static restriction preventing land plays while ten or more lands are on the battlefield are not supported; this card enters the battlefield as an inert enchantment."
    ),
    faces = &[face!(
        name = "Limited Resources",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);
