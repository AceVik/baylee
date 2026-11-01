//! Glowering Rogon — {5}{G} — Creature — Beast
//! Oracle: Amplify 1 (As this creature enters, put a +1/+1 counter on it for each Beast card you reveal in your hand.)
//! Set: LGN #128 — Legions | Scryfall ID: 974b0881-bd26-4074-93dd-a1e3600347c4 | Oracle ID: e9ed7322-53d4-419a-809f-78824aeaf99e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GLOWERING_ROGON,
    oracle_id = "e9ed7322-53d4-419a-809f-78824aeaf99e",
    scryfall_id = "974b0881-bd26-4074-93dd-a1e3600347c4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Glowering Rogon",
        mana_cost = mana!("{5}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
