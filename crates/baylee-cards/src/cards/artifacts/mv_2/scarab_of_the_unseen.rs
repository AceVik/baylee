//! Scarab of the Unseen — {2} — Artifact
//! Oracle: {T}, Sacrifice this artifact: Return all Auras attached to target permanent you own to their owners' hands. Draw a card at the beginning of the next turn's upkeep.
//! Set: ALL #128 — Alliances | Scryfall ID: d5da1c71-6059-4e4e-933d-dbca1cc4bd15 | Oracle ID: 5f085af7-ce66-4019-9516-d1650ee7b6e0
// IMPLEMENTED — {T}, sacrifice: returns every Aura attached to the target permanent you own to its owner's hand, then a card is drawn at the next turn's upkeep.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PERMANENT_YOU_OWN: Filter = Filter::OwnedByYou;
static AURA_ON_TARGET: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::enchantment::AURA),
    Filter::AttachedTo(&Filter::Reference),
]);

card!(
    index = index::SCARAB_OF_THE_UNSEEN,
    oracle_id = "5f085af7-ce66-4019-9516-d1650ee7b6e0",
    scryfall_id = "d5da1c71-6059-4e4e-933d-dbca1cc4bd15",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Scarab of the Unseen",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!(TapSelf, SacrificeSelf),
        &[
            Effect::ReturnAllToHand {
                filter: &AURA_ON_TARGET,
                opponents_only: false,
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        target = Some(TargetSpec::Object(&PERMANENT_YOU_OWN)),
    )],
);
