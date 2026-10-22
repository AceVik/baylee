//! Bone Mask — {4} — Artifact
//! Oracle: {2}, {T}: The next time a source of your choice would deal damage to you this turn, prevent that damage. Exile cards from the top of your library equal to the damage prevented this way.
//! Set: MIR #295 — Mirage | Scryfall ID: 1f46aea3-ea00-46c4-b207-522ceeeae68b | Oracle ID: ddfc7dac-f921-4529-935c-6c588890c522
// NOT SUPPORTED: exiling cards from the top of your library equal to the damage prevented this way.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BONE_MASK,
    oracle_id = "ddfc7dac-f921-4529-935c-6c588890c522",
    scryfall_id = "1f46aea3-ea00-46c4-b207-522ceeeae68b",
    coverage = Coverage::Partial(
        "prevents the next damage a source of your choice would deal to you this turn; exiling cards from the top of your library equal to the damage prevented this way is not supported",
    ),
    faces = &[face!(
        name = "Bone Mask",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}", TapSelf),
        &[Effect::PreventNextFromChosenSource {
            sources: &Filter::Any,
            combat_only: false,
            all_but: 0,
            gain_life: false,
        }],
    )],
);
