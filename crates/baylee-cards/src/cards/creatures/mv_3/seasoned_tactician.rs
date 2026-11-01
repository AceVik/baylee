//! Seasoned Tactician — {2}{W} — Creature — Human Advisor
//! Oracle: {3}, Exile the top four cards of your library: The next time a source of your choice would deal damage to you this turn, prevent that damage.
//! Set: ALL #17 — Alliances | Scryfall ID: f8be4b6b-23a2-42d2-911d-fa14f7f5a95b | Oracle ID: d7c2f27c-8c82-4283-a17d-f979ee636542

// PARTIAL — a vanilla 1/3: the {3} ability is not played.
// NOT SUPPORTED: "{3}, Exile the top four cards of your library: The next time a source of your choice would deal damage to you this turn, prevent that damage." CostPart has no exile-the-top-cards-of-your-library cost; the ability without it would shield you for {3} alone.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SEASONED_TACTICIAN,
    oracle_id = "d7c2f27c-8c82-4283-a17d-f979ee636542",
    scryfall_id = "f8be4b6b-23a2-42d2-911d-fa14f7f5a95b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Seasoned Tactician",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ADVISOR],
        power = Some(1),
        toughness = Some(3),
    ),],
    coverage = Coverage::Partial(
        "The {3}, exile-the-top-four-cards-of-your-library ability is not played: there is no cost for exiling the top cards of a library, and the shield without it would cost {3} alone; Seasoned Tactician enters as a 1/3 with no abilities."
    ),
);
