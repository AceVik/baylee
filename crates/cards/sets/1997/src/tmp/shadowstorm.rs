//! Shadowstorm — {R} — Sorcery
//! Oracle: Shadowstorm deals 2 damage to each creature with shadow.
//! Set: TPR #157 — Tempest Remastered | Scryfall ID: de128174-b174-42cf-b7cd-ddbaf8781500 | Oracle ID: 6e66e983-6349-428b-b511-8738c03ef8fc

use baylee_cards_dsl::prelude::*;

static CREATURE_WITH_SHADOW: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::SHADOW)]);

card!(
    index = index::SHADOWSTORM,
    oracle_id = "6e66e983-6349-428b-b511-8738c03ef8fc",
    scryfall_id = "de128174-b174-42cf-b7cd-ddbaf8781500",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shadowstorm",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::DealDamageEach {
        amount: Amount::Fixed(2),
        filter: &CREATURE_WITH_SHADOW,
    }])],
);
