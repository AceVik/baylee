//! Twining Twins // Swift Spiral — {2}{U}{U} — Creature — Faerie Wizard // Instant — Adventure
//! Oracle: Twining Twins — Flying, vigilance, ward {1}. 4/4.
//! Oracle: Swift Spiral {1}{W} — Instant — Adventure: Exile target nontoken creature. Return it to the battlefield under its owner's control at the beginning of the next end step.
//! Set: EOC #66 — Edge of Eternities Commander | Scryfall ID: 043718ea-59f6-4d1a-94c5-271704c1a38a | Oracle ID: 105aea98-8eb9-4fb2-a0cb-7c7513317c5b
// IMPLEMENTED — flying/vigilance/ward 4/4 front + the adventure back
// (cast Swift Spiral, exile on resolution, cast Twining Twins from
// exile later — CR 715).
// NOTE: data corrected against Scryfall (the stub header had a modal
// ETB from a different card).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static NONTOKEN_CREATURE: Filter = Filter::And(&[
    Filter::HasType(TypeSet::CREATURE),
    Filter::Not(&Filter::IsToken),
]);
static BACK_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[Effect::ExileAndReturnAtEndStep],
    targets: Some(TargetReq::one(TargetSpec::Object(&NONTOKEN_CREATURE))),
}];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(176),
    oracle_id: "105aea98-8eb9-4fb2-a0cb-7c7513317c5b",
    scryfall_id: "043718ea-59f6-4d1a-94c5-271704c1a38a",
    faces: &[
        FaceDef {
            name: "Twining Twins",
            mana_cost: baylee_core::mana!("{2}{U}{U}"),
            types: TypeSet::CREATURE,
            subtypes: &[creature::FAERIE, creature::WIZARD],
            power: Some(4),
            toughness: Some(4),
            ..FaceDef::DEFAULT
        },
        FaceDef {
            name: "Swift Spiral",
            mana_cost: baylee_core::mana!("{1}{W}"),
            types: TypeSet::INSTANT,
            abilities: BACK_ABILITIES,
            adventure: true,
            ..FaceDef::DEFAULT
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
    keywords: KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Ward { mana: 1 }],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
