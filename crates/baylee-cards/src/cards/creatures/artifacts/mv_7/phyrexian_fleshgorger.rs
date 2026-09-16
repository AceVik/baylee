//! Phyrexian Fleshgorger — {7} — Artifact Creature — Phyrexian Wurm
//! Oracle: Prototype {1}{B}{B} — 3/3 (You may cast this spell with different mana cost, color, and size. It keeps its abilities and types.)
//! Oracle: Menace, lifelink
//! Oracle: Ward—Pay life equal to this creature's power.
//! Set: BRO #121 — The Brothers' War | Scryfall ID: 62d37423-3445-412a-9abd-0480da404637 | Oracle ID: d3a5a830-cd14-49da-9412-c50049c74c92
// PARTIAL — the {7} 7/5 body with menace and lifelink is built; prototype and
// the ward cost are not expressible in the DSL and are flagged below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "Prototype {1}{B}{B} — 3/3 (You may cast this spell with
// different mana cost, color, and size.)" — no DSL variant casts a spell for
// an alternative cost that also changes its color and its power/toughness.
// `AlternativeCost` swaps the mana paid and nothing else; `SpellMode`'s
// `cost_override` belongs to a modal spell and carries no characteristics
// either.
// NOT SUPPORTED: "Ward—Pay life equal to this creature's power." —
// `AbilityDef::Ward` takes a fixed amount of generic mana, and this ward is
// paid in life equal to a value computed off the source (7 here, 3 in
// prototype form), which no ward variant carries.

card!(
    index = index::PHYREXIAN_FLESHGORGER,
    oracle_id = "d3a5a830-cd14-49da-9412-c50049c74c92",
    scryfall_id = "62d37423-3445-412a-9abd-0480da404637",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Phyrexian Fleshgorger",
        mana_cost = mana!("{7}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::WURM],
        power = Some(7),
        toughness = Some(5),
        keywords = KeywordSet::MENACE.union(KeywordSet::LIFELINK),
    ),],
    coverage = Coverage::Partial(
        "prototype (an alternative cast with a different mana cost, color and size) and ward—pay life equal to this creature's power"
    ),
);
