//! Mangara's Tome — {5} — Artifact — Book
//! Oracle: When this artifact enters, search your library for five cards, exile them in a face-down pile, and shuffle that pile. Then shuffle your library.
//! Oracle: {2}: The next time you would draw a card this turn, instead put the top card of the exiled pile into its owner's hand.
//! Set: MIR #309 — Mirage | Scryfall ID: 557ebacf-4a8f-4548-a142-533b7adfdac3 | Oracle ID: f515c691-07be-47ea-8bc0-46221eab8b46
// NOT SUPPORTED: exiling five searched cards in a shuffled face-down pile and replacing your next draw with the top card of that pile are not in Effect.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MANGARA_S_TOME,
    oracle_id = "f515c691-07be-47ea-8bc0-46221eab8b46",
    scryfall_id = "557ebacf-4a8f-4548-a142-533b7adfdac3",
    coverage = Coverage::Partial(
        "when it enters, shuffles your library; searching for five cards to exile in a shuffled face-down pile and activating {2} to replace your next draw with the top card of that pile are not in Effect",
    ),
    faces = &[face!(
        name = "Mangara's Tome",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::BOOK],
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::ShuffleLibrary {
            who: PlayerRel::You,
        }],
    )],
);
