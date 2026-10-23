//! Cataclysm — {2}{W}{W} — Sorcery
//! Oracle: Each player chooses from among the permanents they control an artifact, a creature, an enchantment, and a land, then sacrifices the rest.
//! Set: TPR #8 — Tempest Remastered | Scryfall ID: 713160be-bcfd-4ef5-9457-c02b926ba06e | Oracle ID: b0479ca4-e925-4aa7-a4fa-f7e01f899ac5

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CATACLYSM,
    oracle_id = "b0479ca4-e925-4aa7-a4fa-f7e01f899ac5",
    scryfall_id = "713160be-bcfd-4ef5-9457-c02b926ba06e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Choosing one artifact, one creature, one enchantment, and one land per player and sacrificing the rest is not supported; resolves as a no-op."
    ),
    faces = &[face!(
        name = "Cataclysm",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[])],
);
