//! Noble Benefactor — {2}{U} — Creature — Human Cleric
//! Oracle: When this creature dies, each player may search their library for a card and put that card into their hand. Then each player who searched their library this way shuffles.
//! Set: WTH #44 — Weatherlight | Scryfall ID: bd221f30-1773-4e05-a40f-022a9306ef89 | Oracle ID: 78c1d6fe-38e4-40c4-9252-87ff272247ad
// PARTIAL — When this creature dies, each player searches their library for a card, puts it into their hand, and shuffles; making each player's library search optional ('may search') is not yet modeled on EachPlayerSearchBeforeShuffle.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NOBLE_BENEFACTOR,
    oracle_id = "78c1d6fe-38e4-40c4-9252-87ff272247ad",
    scryfall_id = "bd221f30-1773-4e05-a40f-022a9306ef89",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "When this creature dies, each player searches their library for a card, puts it into their hand, and shuffles; making each player's library search optional ('may search') is not yet modeled on EachPlayerSearchBeforeShuffle.",
    ),
    faces = &[face!(
        name = "Noble Benefactor",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[
            Effect::EachPlayerSearchBeforeShuffle {
                filter: &Filter::Any,
                find: &Find::HAND,
            },
            Effect::ShuffleLibrary {
                who: PlayerRel::EachPlayer,
            },
        ],
    ),],
);
