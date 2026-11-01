//! Bola Warrior — {1}{R} — Creature — Human Spellshaper Warrior
//! Oracle: {R}, {T}, Discard a card: Target creature can't block this turn.
//! Set: NEM #78 — Nemesis | Scryfall ID: dc6e1de6-e7e0-4037-896a-f80c54b8ef5c | Oracle ID: 13f41253-364f-40a5-a7cd-84d8785b2460
// IMPLEMENTED — spellshaper activation granting CANT_BLOCK until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BOLA_WARRIOR,
    oracle_id = "13f41253-364f-40a5-a7cd-84d8785b2460",
    scryfall_id = "dc6e1de6-e7e0-4037-896a-f80c54b8ef5c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bola Warrior",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::SPELLSHAPER,
            subtypes::creature::WARRIOR
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{R}", TapSelf, Discard(&Filter::Any)),
        &[Effect::PumpTarget {
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::CANT_BLOCK,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    ),],
);
