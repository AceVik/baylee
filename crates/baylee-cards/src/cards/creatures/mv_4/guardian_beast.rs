//! Guardian Beast — {3}{B} — Creature — Beast
//! Oracle: As long as this creature is untapped, noncreature artifacts you control can't be enchanted, they have indestructible, and other players can't gain control of them. This effect doesn't remove Auras already attached to those artifacts.
//! Set: ME4 #85 — Masters Edition IV | Scryfall ID: 68f362bb-6391-49de-bce6-2682ec100b6b | Oracle ID: 34c3aebe-a224-426a-ae06-f3145193313e
// PARTIAL — only the untapped-gated indestructible grant is written; the
// no-enchanting and no-control-change clauses are off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GUARDIAN_BEAST,
    oracle_id = "34c3aebe-a224-426a-ae06-f3145193313e",
    scryfall_id = "68f362bb-6391-49de-bce6-2682ec100b6b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Guardian Beast",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(2),
        toughness = Some(4),
    ),],
    coverage = Coverage::Partial(
        "no modifier can stop new Auras without removing already-attached \
         ones, and none refuses a control change, so two of the three \
         clauses are off the card"
    ),
    // NOT SUPPORTED: "noncreature artifacts you control can't be enchanted…
    // This effect doesn't remove Auras already attached to those artifacts."
    // — `Modifier::CantBeEnchantedExceptSource` is the closest piece and is
    // not it: the attachment state-based action reads it for Auras already
    // attached (Consecrate Land's test removes them), while this sentence
    // keeps them. "other players can't gain control of them" has no piece at
    // all: `Modifier::GainControl` takes a permanent and nothing refuses
    // one. Only the indestructible grant is below.
    abilities = &[static_ability!(
        Filter::And(&[
            Filter::ARTIFACT,
            Filter::NONCREATURE,
            Filter::ControlledByYou,
        ]),
        Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE),
        condition = Some(Condition::SourceMatches(&Filter::Untapped)),
    )],
);
