//! Gerrard's Battle Cry — {W} — Enchantment
//! Oracle: {2}{W}: Creatures you control get +1/+1 until end of turn.
//! Set: TPR #15 — Tempest Remastered | Scryfall ID: 814c4295-5d99-45da-87cc-29bb1b774e99 | Oracle ID: 12ef651c-5a86-49c4-a512-292e9d135a76
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GERRARD_S_BATTLE_CRY,
    oracle_id = "12ef651c-5a86-49c4-a512-292e9d135a76",
    scryfall_id = "814c4295-5d99-45da-87cc-29bb1b774e99",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gerrard's Battle Cry",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{2}{W}"),
        &[Effect::PumpFilter {
            filter: &Filter::YOUR_CREATURE,
            controlled_by: None,
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(1),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn
        }]
    ),],
);
