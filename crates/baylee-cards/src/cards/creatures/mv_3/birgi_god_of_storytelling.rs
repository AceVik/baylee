//! Birgi, God of Storytelling // Harnfel, Horn of Bounty — {2}{R} — Legendary Creature — God // Legendary Artifact
//! Oracle: Whenever you cast a spell, add {R}. Until end of turn, you don't lose this mana as steps and phases end.
//! Oracle: Creatures you control can boast twice during each of your turns rather than once.
//! Oracle: Discard a card: Exile the top two cards of your library. You may play those cards this turn.
//! Set: KHM #123 — Kaldheim | Scryfall ID: 44657ab1-0a6a-4a5f-9688-86f239083821 | Oracle ID: fb81e4d3-1d8c-4779-be62-87cf49277e51
//! Face: Birgi, God of Storytelling — {2}{R} — Legendary Creature — God
//! Face: Harnfel, Horn of Bounty — {4}{R} — Legendary Artifact
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BIRGI_GOD_OF_STORYTELLING,
    oracle_id = "fb81e4d3-1d8c-4779-be62-87cf49277e51",
    scryfall_id = "44657ab1-0a6a-4a5f-9688-86f239083821",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[
        face!(
            name = "Birgi, God of Storytelling",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::GOD],
            power = Some(3),
            toughness = Some(3),
        ),
        face!(
            name = "Harnfel, Horn of Bounty",
            mana_cost = mana!("{4}{R}"),
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
