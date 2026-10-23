//! Memory Jar — {5} — Artifact
//! Oracle: {T}, Sacrifice this artifact: Each player exiles all cards from their hand face down and draws seven cards. At the beginning of the next end step, each player discards their hand and returns to their hand each card they exiled this way.
//! Set: VMA #276 — Vintage Masters | Scryfall ID: 60b70b7f-6f24-4c00-947a-b109f302205b | Oracle ID: eb9c9a34-88a8-4afe-a053-806f6b29834f
// PARTIAL

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MEMORY_JAR,
    oracle_id = "eb9c9a34-88a8-4afe-a053-806f6b29834f",
    scryfall_id = "60b70b7f-6f24-4c00-947a-b109f302205b",
    coverage = Coverage::Partial(
        "Activating {T}, Sacrifice this artifact causes each player to discard their hand and draw seven cards; exiling each player's hand face down and returning those cards at the next end step after discarding is not yet in Effect.",
    ),
    faces = &[face!(
        name = "Memory Jar",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!(TapSelf, SacrificeSelf),
        &[
            Effect::DiscardHand {
                who: PlayerRel::EachPlayer,
            },
            Effect::DrawCardsFor {
                who: PlayerRel::EachPlayer,
                amount: Amount::Fixed(7),
            },
        ],
    )],
);
