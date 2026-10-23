//! Silver Wyvern — {3}{U}{U} — Creature — Drake
//! Oracle: Flying
//! Oracle: {U}: Change the target of target spell or ability that targets only this creature. The new target must be a creature.
//! Set: TPR #68 — Tempest Remastered | Scryfall ID: 882dc8fc-0938-4619-807d-bb60df8ac390 | Oracle ID: 4baa8f05-1ef8-4770-8c66-ef5f0f02a717
// PARTIAL — Redirects a single-target spell targeting this creature to a creature; redirecting abilities targeting this creature is not supported

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SILVER_WYVERN,
    oracle_id = "4baa8f05-1ef8-4770-8c66-ef5f0f02a717",
    scryfall_id = "882dc8fc-0938-4619-807d-bb60df8ac390",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Redirects a single-target spell targeting this creature to a creature; redirecting abilities targeting this creature is not supported"
    ),
    keywords = KeywordSet::FLYING,
    faces = &[face!(
        name = "Silver Wyvern",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAKE],
        power = Some(4),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{U}"),
        &[Effect::ChangeTarget {
            to: &Filter::CREATURE,
        }],
        target = Some(TargetSpec::Spell(&Filter::And(&[
            Filter::WithSingleTarget,
            Filter::SpellTargetingAffected
        ]))),
    ),],
);
