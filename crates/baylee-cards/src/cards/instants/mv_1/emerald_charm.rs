//! Emerald Charm — {G} — Instant
//! Oracle: Choose one —
//! Oracle: • Untap target permanent.
//! Oracle: • Destroy target non-Aura enchantment.
//! Oracle: • Target creature loses flying until end of turn.
//! Set: DMR #158 — Dominaria Remastered | Scryfall ID: b5862e38-b48b-45e1-b0f6-d1d0d1124e50 | Oracle ID: 904b3db5-70bc-4d49-9887-f504aff8666f
// IMPLEMENTED — modal instant: untap target permanent, destroy target non-Aura enchantment, or target creature loses flying until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NON_AURA_ENCHANTMENT: Filter = Filter::And(&[
    Filter::ENCHANTMENT,
    Filter::Not(&Filter::HasSubtype(subtypes::enchantment::AURA)),
]);

card!(
    index = index::EMERALD_CHARM,
    oracle_id = "904b3db5-70bc-4d49-9887-f504aff8666f",
    scryfall_id = "b5862e38-b48b-45e1-b0f6-d1d0d1124e50",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Emerald Charm",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[Effect::UntapTarget],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
            ),
            mode!(
                &[Effect::destroy(TargetSpec::Object(&NON_AURA_ENCHANTMENT))],
                targets = Some(TargetReq::one(TargetSpec::Object(&NON_AURA_ENCHANTMENT))),
            ),
            mode!(
                &[Effect::ModifyTargets {
                    modifier: Modifier::RemoveKeyword(KeywordSet::FLYING),
                    duration: Duration::UntilEndOfTurn,
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
            ),
        ],
    }],
);
