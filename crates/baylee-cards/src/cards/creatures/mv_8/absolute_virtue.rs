//! Absolute Virtue — {6}{W}{U} — Legendary Creature — Avatar Warrior
//! Oracle: This spell can't be countered.
//! Oracle: Flying
//! Oracle: You have protection from each of your opponents. (You can't be dealt damage, enchanted, or targeted by anything controlled by your opponents.)
//! Set: FIN #212 — Final Fantasy | Scryfall ID: aa192912-c9ee-403f-8a46-a338c9edb4b9 | Oracle ID: d4fddf20-6b3a-42c3-a245-002dacbb4725
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ABSOLUTE_VIRTUE,
    oracle_id = "d4fddf20-6b3a-42c3-a245-002dacbb4725",
    scryfall_id = "aa192912-c9ee-403f-8a46-a338c9edb4b9",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLYING.union(KeywordSet::UNCOUNTERABLE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Absolute Virtue",
        mana_cost = mana!("{6}{W}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::AVATAR, subtypes::creature::WARRIOR],
        power = Some(8),
        toughness = Some(8),
    ),],
    abilities = &[static_ability!(
        Filter::Any,
        Modifier::PlayerProtectionFromOpponents {
            who: PlayerRel::You,
        },
    )],
);
