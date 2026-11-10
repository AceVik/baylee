//! Tinder Wall — {G} — Creature — Plant Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Sacrifice this creature: Add {R}{R}.
//! Oracle: {R}, Sacrifice this creature: It deals 2 damage to target creature it's blocking.
//! Set: ME2 #184 — Masters Edition II | Scryfall ID: 1e62598f-0a91-4cfd-9a28-c3bda61c9ead | Oracle ID: dd2f698c-b3b4-4dc8-9950-9be03ca7dc9e

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLOCKED_BY_SOURCE: Filter = Filter::And(&[Filter::CREATURE, Filter::BlockedBySource]);

card!(
    index = index::TINDER_WALL,
    oracle_id = "dd2f698c-b3b4-4dc8-9950-9be03ca7dc9e",
    scryfall_id = "1e62598f-0a91-4cfd-9a28-c3bda61c9ead",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    coverage = Coverage::Implemented,
    keywords = KeywordSet::DEFENDER,
    faces = &[face!(
        name = "Tinder Wall",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PLANT, subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(3),
    ),],
    abilities = &[
        mana_ability!(cost!(SacrificeSelf), &[Effect::mana(ManaColor::Red, 2)]),
        activated!(
            cost!("{R}", SacrificeSelf),
            &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::Object(&BLOCKED_BY_SOURCE),
            }],
            target = Some(TargetSpec::Object(&BLOCKED_BY_SOURCE))
        ),
    ],
);
