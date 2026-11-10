//! Seedling Charm — {G} — Instant
//! Oracle: Choose one —
//! Oracle: • Return target Aura attached to a creature to its owner's hand.
//! Oracle: • Regenerate target green creature.
//! Oracle: • Target creature gains trample until end of turn.
//! Set: MIR #240 — Mirage | Scryfall ID: e42746e1-f422-4453-a860-3993d5796479 | Oracle ID: b8d66cd6-15db-41ff-bae3-60f1aad3ea23

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static AURA_ON_CREATURE: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::enchantment::AURA),
    Filter::AttachedTo(&Filter::CREATURE),
]);
static GREEN_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
]);

card!(
    index = index::SEEDLING_CHARM,
    oracle_id = "b8d66cd6-15db-41ff-bae3-60f1aad3ea23",
    scryfall_id = "e42746e1-f422-4453-a860-3993d5796479",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Seedling Charm",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[Effect::ReturnToHand {
                    target: TargetSpec::Object(&AURA_ON_CREATURE),
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(&AURA_ON_CREATURE))),
            ),
            mode!(
                &[Effect::regenerate(TargetSpec::Object(&GREEN_CREATURE))],
                targets = Some(TargetReq::one(TargetSpec::Object(&GREEN_CREATURE))),
            ),
            mode!(
                &[Effect::ModifyTargets {
                    modifier: Modifier::AddKeyword(KeywordSet::TRAMPLE),
                    duration: Duration::UntilEndOfTurn,
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
            ),
        ],
    }],
);
