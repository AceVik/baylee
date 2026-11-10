//! Coiling Woodworm — {2}{G} — Creature — Insect Worm
//! Oracle: Coiling Woodworm's power is equal to the number of Forests on the battlefield.
//! Set: NEM #103 — Nemesis | Scryfall ID: 341a70be-2dbf-4365-9c7a-e52cb62a74fa | Oracle ID: 75dcdf05-b392-4a96-98ae-5ec4cd425eee
// IMPLEMENTED — characteristic-defining power equal to all Forests on the battlefield.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static FOREST: Filter = Filter::HasSubtype(subtypes::land::FOREST);

card!(
    index = index::COILING_WOODWORM,
    oracle_id = "75dcdf05-b392-4a96-98ae-5ec4cd425eee",
    scryfall_id = "341a70be-2dbf-4365-9c7a-e52cb62a74fa",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Coiling Woodworm",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT, subtypes::creature::WORM],
        power = Some(0),
        toughness = Some(1),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPTSeparate {
            power: PtCount::OnBattlefield(&FOREST),
            toughness: PtCount::Fixed(1),
        }
    ),],
);
