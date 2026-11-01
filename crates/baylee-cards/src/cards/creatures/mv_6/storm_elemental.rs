//! Storm Elemental — {5}{U} — Creature — Elemental
//! Oracle: Flying
//! Oracle: {U}, Exile the top card of your library: Tap target creature with flying.
//! Oracle: {U}, Exile the top card of your library: If the exiled card is a snow land, this creature gets +1/+1 until end of turn.
//! Set: ME2 #68 — Masters Edition II | Scryfall ID: 2903c66e-056d-4ab6-9dfe-81b18a8da454 | Oracle ID: f1fee486-660e-4356-896e-671e8b675ad8

// PARTIAL — a 3/4 flyer: neither {U} ability is played.
// NOT SUPPORTED: "{U}, Exile the top card of your library: Tap target creature with flying." and "{U}, Exile the top card of your library: If the exiled card is a snow land, this creature gets +1/+1 until end of turn." CostPart has no exile-the-top-card-of-your-library cost, so the first would tap a flyer for {U} alone, and no condition reads the exiled card (a snow land), so the second would pump unconditionally.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STORM_ELEMENTAL,
    oracle_id = "f1fee486-660e-4356-896e-671e8b675ad8",
    scryfall_id = "2903c66e-056d-4ab6-9dfe-81b18a8da454",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Storm Elemental",
        mana_cost = mana!("{5}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        keywords = KeywordSet::FLYING,
        power = Some(3),
        toughness = Some(4),
    ),],
    coverage = Coverage::Partial(
        "Neither {U}, exile-the-top-card-of-your-library ability is played: there is no cost for exiling the top card of a library, and no condition reads whether the exiled card is a snow land; Storm Elemental enters as a 3/4 flyer with no activated abilities."
    ),
);
