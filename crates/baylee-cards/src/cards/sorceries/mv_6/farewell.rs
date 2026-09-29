//! Farewell — {4}{W}{W} — Sorcery
//! Oracle: Choose one or more —
//! Oracle: • Exile all artifacts.
//! Oracle: • Exile all creatures.
//! Oracle: • Exile all enchantments.
//! Oracle: • Exile all graveyards.
//! Set: MKC #64 — Murders at Karlov Manor Commander | Scryfall ID: 114d2180-093b-4838-97ad-badbc8ee50b0 | Oracle ID: 4eb813fd-2d5a-4b02-8193-662681ef4e7d

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FAREWELL,
    oracle_id = "4eb813fd-2d5a-4b02-8193-662681ef4e7d",
    scryfall_id = "114d2180-093b-4838-97ad-badbc8ee50b0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Farewell",
        mana_cost = mana!("{4}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    // "Choose one or more" (CR 700.2d): the chosen modes happen in the
    // order they are printed (CR 608.2c), whichever was picked first.
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE_OR_MORE,
        modes: &[
            mode!(&[Effect::ExileAll {
                filter: &Filter::ARTIFACT
            }]),
            mode!(&[Effect::ExileAll {
                filter: &Filter::CREATURE
            }]),
            mode!(&[Effect::ExileAll {
                filter: &Filter::ENCHANTMENT
            }]),
            mode!(&[Effect::ExileGraveyard {
                player: PlayerRel::EachPlayer
            }]),
        ],
    }],
);
