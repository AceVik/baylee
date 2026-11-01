//! Llanowar Sentinel — {2}{G} — Creature — Elf
//! Oracle: When this creature enters, you may pay {1}{G}. If you do, search your library for a card named Llanowar Sentinel, put that card onto the battlefield, then shuffle.
//! Set: 10E #275 — Tenth Edition | Scryfall ID: bda18561-4767-4f8e-bc6d-9117d1a480b1 | Oracle ID: 272ab5d5-7edb-4457-8a09-cabe2325c881
// IMPLEMENTED — hand-authored in `baylee-cards`.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SENTINEL_CARD: Filter = Filter::Named("Llanowar Sentinel");

card!(
    index = index::LLANOWAR_SENTINEL,
    oracle_id = "272ab5d5-7edb-4457-8a09-cabe2325c881",
    scryfall_id = "bda18561-4767-4f8e-bc6d-9117d1a480b1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Llanowar Sentinel",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::PlayerMayPayManaThen {
            player: PlayerRel::You,
            cost: mana!("{1}{G}"),
            effects: &[Effect::SearchLibrary {
                filter: &SENTINEL_CARD,
                finds: &[Find::BATTLEFIELD],
                optional: true
            }]
        }]
    ),],
);
