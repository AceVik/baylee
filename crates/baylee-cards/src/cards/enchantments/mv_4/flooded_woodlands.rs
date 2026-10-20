//! Flooded Woodlands — {2}{U}{B} — Enchantment
//! Oracle: Green creatures can't attack unless their controller sacrifices a land of their choice for each green creature they control that's attacking. (This cost is paid as attackers are declared.)
//! Set: ICE #290 — Ice Age | Scryfall ID: de89e9e1-485b-42e5-9728-5d6f948999e1 | Oracle ID: 413172aa-b508-4e67-b68e-6a749d2e8130
// PARTIAL: sacrificing a land per attacking green creature as an attack declaration cost not supported.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: Green creatures can't attack unless their controller sacrifices a land of their choice for each green creature they control that's attacking.
card!(
    index = index::FLOODED_WOODLANDS,
    oracle_id = "413172aa-b508-4e67-b68e-6a749d2e8130",
    scryfall_id = "de89e9e1-485b-42e5-9728-5d6f948999e1",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Partial(
        "sacrificing a land per attacking green creature as an attack declaration cost not supported"
    ),
    faces = &[face!(
        name = "Flooded Woodlands",
        mana_cost = mana!("{2}{U}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);
