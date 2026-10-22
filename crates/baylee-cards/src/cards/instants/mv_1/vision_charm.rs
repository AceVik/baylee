//! Vision Charm — {U} — Instant
//! Oracle: Choose one —
//! Oracle: • Target player mills four cards.
//! Oracle: • Choose a land type and a basic land type. Each land of the first chosen type becomes the second chosen type until end of turn.
//! Oracle: • Target artifact phases out. (While it's phased out, it's treated as though it doesn't exist. It phases in before its controller untaps during their next untap step.)
//! Set: VIS #49 — Visions | Scryfall ID: 78b384d3-3adf-493a-8b89-bfe68fd1c3e2 | Oracle ID: c56d2c5b-c3e3-4236-bc38-286e250aa4f7
// PARTIAL: modes 1 and 3 work as printed (target player mills four cards, or target artifact phases out); mode 2 sets all lands to Islands until end of turn because choosing a land type and a basic land type on resolution is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VISION_CHARM,
    oracle_id = "c56d2c5b-c3e3-4236-bc38-286e250aa4f7",
    scryfall_id = "78b384d3-3adf-493a-8b89-bfe68fd1c3e2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "modes 1 and 3 work as printed (target player mills four cards, or target artifact phases out); mode 2 sets all lands to Islands until end of turn because choosing a land type and a basic land type on resolution is not yet modeled",
    ),
    faces = &[face!(
        name = "Vision Charm",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[Effect::Mill {
                    amount: Amount::Fixed(4),
                    target: PlayerRel::Chosen,
                }],
                targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
            ),
            mode!(&[Effect::continuous(
                &Filter::LAND,
                Modifier::SetLandType(subtypes::land::ISLAND),
                Duration::UntilEndOfTurn,
            )]),
            mode!(
                &[Effect::PhaseOut {
                    target: Some(TargetSpec::Object(&Filter::ARTIFACT)),
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))),
            ),
        ],
    }],
);
