//! Hearth Charm — {R} — Instant
//! Oracle: Choose one —
//! Oracle: • Destroy target artifact creature.
//! Oracle: • Attacking creatures get +1/+0 until end of turn.
//! Oracle: • Target creature with power 2 or less can't be blocked this turn.
//! Set: VIS #82 — Visions | Scryfall ID: caa9ac66-51b7-4aec-92dc-0f0656b0f7fe | Oracle ID: 4ce95005-82b3-4921-b1c4-60d775a012a2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static ARTIFACT_CREATURE: Filter = Filter::And(&[Filter::ARTIFACT, Filter::CREATURE]);
static ATTACKING_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Attacking]);
static SMALL_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::PowerAtMost(2)]);

card!(
    index = index::HEARTH_CHARM,
    oracle_id = "4ce95005-82b3-4921-b1c4-60d775a012a2",
    scryfall_id = "caa9ac66-51b7-4aec-92dc-0f0656b0f7fe",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hearth Charm",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[Effect::destroy(TargetSpec::Object(&ARTIFACT_CREATURE))],
                targets = Some(TargetReq::one(TargetSpec::Object(&ARTIFACT_CREATURE))),
            ),
            mode!(&[Effect::continuous(
                &ATTACKING_CREATURE,
                Modifier::ModifyPT(1, 0),
                Duration::UntilEndOfTurn,
            )]),
            mode!(
                &[Effect::PumpTarget {
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::UNBLOCKABLE,
                    duration: Duration::UntilEndOfTurn,
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(&SMALL_CREATURE))),
            ),
        ],
    }],
);
