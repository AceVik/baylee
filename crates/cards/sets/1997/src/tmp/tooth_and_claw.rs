//! Tooth and Claw — {3}{R} — Enchantment
//! Oracle: Sacrifice two creatures: Create a 3/1 red Beast creature token named Carnivore.
//! Set: EMA #151 — Eternal Masters | Scryfall ID: a8c77d9f-22bd-4676-b594-8499369c449c | Oracle ID: 587368eb-068c-44a3-ba8c-5ad0f59f880f

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::TOOTH_AND_CLAW,
    oracle_id = "587368eb-068c-44a3-ba8c-5ad0f59f880f",
    scryfall_id = "a8c77d9f-22bd-4676-b594-8499369c449c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tooth and Claw",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!(
            Sacrifice(&Filter::YOUR_CREATURE),
            Sacrifice(&Filter::YOUR_CREATURE)
        ),
        &[Effect::CreateToken {
            token: &generated_tokens::CARNIVORE_3_1_RED,
        }],
    )],
);
