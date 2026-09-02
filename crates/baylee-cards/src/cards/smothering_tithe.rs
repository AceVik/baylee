//! Smothering Tithe — {3}{W} — Enchantment
//! Oracle: Whenever an opponent draws a card, that player may pay {2}. If they don't, you create a Treasure token. (It's an artifact with "{T}, Sacrifice this artifact: Add one mana of any color.")
//! Set: 2X2 #32 — Double Masters 2022 | Scryfall ID: 861b5889-0183-4bee-afeb-a4b2aa700a8e | Oracle ID: 153376c9-dffd-458c-8ce3-a4c8269bc4e9
// IMPLEMENTED — opponent-choice {2} tax → Treasure tokens.

use baylee_cards_dsl::prelude::*;

use crate::tokens::TREASURE as TREASURE_TOKEN;
static MAKE_TREASURE: Effect = Effect::CreateToken {
    token: &TREASURE_TOKEN,
};

card! {
    index: 147,
    oracle_id: "153376c9-dffd-458c-8ce3-a4c8269bc4e9",
    scryfall_id: "861b5889-0183-4bee-afeb-a4b2aa700a8e",
    faces: &[face! {
        name: "Smothering Tithe",
        mana_cost: baylee_core::mana!("{3}{W}"),
        types: TypeSet::ENCHANTMENT,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::Draws(PlayerRel::Opponent), &[Effect::PlayerMayPayOr {
            player: PlayerRel::Opponent,
            mana: 2,
            effect: &MAKE_TREASURE,
        }])],
}
