//! Jester's Cap — {4} — Artifact
//! Oracle: {2}, {T}, Sacrifice this artifact: Search target player's library for three cards and exile them. Then that player shuffles.
//! Set: DMR #227 — Dominaria Remastered | Scryfall ID: b364e7e7-83e5-474b-9768-8b70a77e5ffa | Oracle ID: 8783a94a-d0b2-4291-a550-dbcf8e16bf4b
// NOT SUPPORTED: SearchDest has no Exile variant yet, so the activated ability pays {2}, {T}, sacrifices Jester's Cap, and shuffles target player's library without searching for and exiling three cards.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JESTER_S_CAP,
    oracle_id = "8783a94a-d0b2-4291-a550-dbcf8e16bf4b",
    scryfall_id = "b364e7e7-83e5-474b-9768-8b70a77e5ffa",
    coverage = Coverage::Partial(
        "SearchDest has no Exile variant yet, so the activated ability pays {2}, {T}, sacrifices Jester's Cap, and shuffles target player's library without searching for and exiling three cards",
    ),
    faces = &[face!(
        name = "Jester's Cap",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}", TapSelf, SacrificeSelf),
        &[Effect::ShuffleLibrary {
            who: PlayerRel::Chosen,
        }],
        target = Some(TargetSpec::AnyPlayer),
    )],
);
