//! Illusion // Reality — {U} — Instant // Instant
//! Oracle: Target spell or permanent becomes the color of your choice until end of turn.
//! Oracle: Destroy target artifact.
//! Set: DMR #213 — Dominaria Remastered | Scryfall ID: ba6ea373-65ea-45a8-8fed-31c482ba8387 | Oracle ID: 5af2689d-c45c-4d45-b6a9-f32e8410b336
//! Face: Illusion — {U} — Instant
//! Face: Reality — {2}{G} — Instant
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ILLUSION,
    oracle_id = "5af2689d-c45c-4d45-b6a9-f32e8410b336",
    scryfall_id = "ba6ea373-65ea-45a8-8fed-31c482ba8387",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[
        face!(
            name = "Illusion",
            mana_cost = mana!("{U}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Reality",
            mana_cost = mana!("{2}{G}"),
            types = TypeSet::INSTANT,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
