//! Downdraft — {2}{G} — Enchantment
//! Oracle: {G}: Target creature loses flying until end of turn.
//! Oracle: Sacrifice this enchantment: It deals 2 damage to each creature with flying.
//! Set: WTH #125 — Weatherlight | Scryfall ID: ab4ced80-926a-4e4d-8ebd-d4fe7374a6ad | Oracle ID: c2508190-5aca-4511-adc8-2072cd228119
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static CREATURE_WITH_FLYING: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]);

card!(
    index = index::DOWNDRAFT,
    oracle_id = "c2508190-5aca-4511-adc8-2072cd228119",
    scryfall_id = "ab4ced80-926a-4e4d-8ebd-d4fe7374a6ad",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Downdraft",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        activated!(
            cost!("{G}"),
            &[Effect::ModifyTargets {
                modifier: Modifier::RemoveKeyword(KeywordSet::FLYING),
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
        activated!(
            cost!(SacrificeSelf),
            &[Effect::damage_each(2, &CREATURE_WITH_FLYING)],
        ),
    ],
);
