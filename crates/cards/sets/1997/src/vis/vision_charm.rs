//! Vision Charm — {U} — Instant
//! Oracle: Choose one —
//! Oracle: • Target player mills four cards.
//! Oracle: • Choose a land type and a basic land type. Each land of the first chosen type becomes the second chosen type until end of turn.
//! Oracle: • Target artifact phases out. (While it's phased out, it's treated as though it doesn't exist. It phases in before its controller untaps during their next untap step.)
//! Set: VIS #49 — Visions | Scryfall ID: 78b384d3-3adf-493a-8b89-bfe68fd1c3e2 | Oracle ID: c56d2c5b-c3e3-4236-bc38-286e250aa4f7
// PARTIAL: the mill mode (target player mills four cards) and the phase-out mode (target artifact phases out) work as printed; the land-type mode is left out because choosing a land type and a basic land type on resolution is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VISION_CHARM,
    oracle_id = "c56d2c5b-c3e3-4236-bc38-286e250aa4f7",
    scryfall_id = "78b384d3-3adf-493a-8b89-bfe68fd1c3e2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the mill mode (target player mills four cards) and the phase-out mode (target artifact phases out) work as printed; the land-type mode is left out because choosing a land type and a basic land type on resolution is not yet modeled",
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
            // NOT SUPPORTED: "Choose a land type and a basic land type. Each land of the first
            // chosen type becomes the second chosen type until end of turn." There is no choice
            // of a land type on resolution; a mode that turned every land into an Island would
            // be a different card.
            mode!(
                &[Effect::PhaseOut {
                    target: Some(TargetSpec::Object(&Filter::ARTIFACT)),
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))),
            ),
        ],
    }],
);
