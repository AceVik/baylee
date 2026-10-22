//! Scarab of the Unseen — {2} — Artifact
//! Oracle: {T}, Sacrifice this artifact: Return all Auras attached to target permanent you own to their owners' hands. Draw a card at the beginning of the next turn's upkeep.
//! Set: ALL #128 — Alliances | Scryfall ID: d5da1c71-6059-4e4e-933d-dbca1cc4bd15 | Oracle ID: 5f085af7-ce66-4019-9516-d1650ee7b6e0
// NOT SUPPORTED: returning only the Auras attached to the specific targeted permanent you own requires a target-relative attachment filter.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PERMANENT_YOU_OWN: Filter = Filter::OwnedByYou;
static AURA_ON_OWNED: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::enchantment::AURA),
    Filter::AttachedTo(&PERMANENT_YOU_OWN),
]);

card!(
    index = index::SCARAB_OF_THE_UNSEEN,
    oracle_id = "5f085af7-ce66-4019-9516-d1650ee7b6e0",
    scryfall_id = "d5da1c71-6059-4e4e-933d-dbca1cc4bd15",
    coverage = Coverage::Partial(
        "Returning only the Auras attached to the specific targeted permanent you own requires a target-relative attachment filter; in play, Scarab of the Unseen targets a permanent you own, returns all Auras attached to permanents you own to their owners' hands, and draws a card at the beginning of the next turn's upkeep.",
    ),
    faces = &[face!(
        name = "Scarab of the Unseen",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!(TapSelf, SacrificeSelf),
        &[
            Effect::ReturnAllToHand {
                filter: &AURA_ON_OWNED,
                opponents_only: false,
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        target = Some(TargetSpec::Object(&PERMANENT_YOU_OWN)),
    )],
);
