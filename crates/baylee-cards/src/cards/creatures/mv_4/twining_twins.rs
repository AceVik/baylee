//! Twining Twins // Swift Spiral — {2}{U}{U} — Creature — Faerie Wizard // Instant — Adventure
//! Oracle: Flying, vigilance, ward {1}
//! Oracle: Exile target nontoken creature. Return it to the battlefield under its owner's control at the beginning of the next end step.
//! Set: WOE #240 — Wilds of Eldraine | Scryfall ID: 043718ea-59f6-4d1a-94c5-271704c1a38a | Oracle ID: 105aea98-8eb9-4fb2-a0cb-7c7513317c5b
// IMPLEMENTED — flying/vigilance/ward 4/4 front + the adventure back
// (cast Swift Spiral, exile on resolution, cast Twining Twins from
// exile later — CR 715).
// NOTE: data corrected against Scryfall (the stub header had a modal
// ETB from a different card).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{creature, spell};

static NONTOKEN_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::IsToken)]);
static BACK_ABILITIES: &[AbilityDef] = &[spell!(
    &[Effect::ExileAndReturnAtEndStep],
    targets = Some(TargetReq::one(TargetSpec::Object(&NONTOKEN_CREATURE)))
)];

card!(
    index = index::TWINING_TWINS,
    oracle_id = "105aea98-8eb9-4fb2-a0cb-7c7513317c5b",
    scryfall_id = "043718ea-59f6-4d1a-94c5-271704c1a38a",
    faces = &[
        face!(
            name = "Twining Twins",
            mana_cost = mana!("{2}{U}{U}"),
            types = TypeSet::CREATURE,
            subtypes = &[creature::FAERIE, creature::WIZARD],
            power = Some(4),
            toughness = Some(4),
        ),
        face!(
            name = "Swift Spiral",
            mana_cost = mana!("{1}{W}"),
            types = TypeSet::INSTANT,
            subtypes = &[spell::ADVENTURE],
            abilities = BACK_ABILITIES,
            adventure = true,
        ),
    ],
    color_identity = ColorSet::from_slice(&[Color::White, Color::Blue]),
    keywords = KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
    coverage = Coverage::Implemented,
    abilities = &[AbilityDef::Ward { mana: 1 }],
);
