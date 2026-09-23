//! Dragonback Assault — {3}{G}{U}{R} — Enchantment
//! Oracle: When this enchantment enters, it deals 3 damage to each creature and each planeswalker.
//! Oracle: Landfall — Whenever a land you control enters, create a 4/4 red Dragon creature token with flying.
//! Set: TDM #179 — Tarkir: Dragonstorm | Scryfall ID: d54cc838-d79d-433a-99fb-d6e4d1c1431d | Oracle ID: 413fb2db-f1a1-4d22-ac37-a52821d35ca2
use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::DRAGONBACK_ASSAULT,
    oracle_id = "413fb2db-f1a1-4d22-ac37-a52821d35ca2",
    scryfall_id = "d54cc838-d79d-433a-99fb-d6e4d1c1431d",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Dragonback Assault",
        mana_cost = mana!("{3}{G}{U}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::damage_each(3, &Filter::CREATURE_OR_PLANESWALKER)]
        ),
        triggered!(
            Trigger::EntersBattlefield(&Filter::YOUR_LAND),
            &[Effect::CreateToken {
                token: &generated_tokens::DRAGON_4_4_RED_FLYING,
            }]
        ),
    ],
);
