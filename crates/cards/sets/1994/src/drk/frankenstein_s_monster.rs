//! Frankenstein's Monster — {X}{B}{B} — Creature — Zombie
//! Oracle: As this creature enters, exile X creature cards from your graveyard. If you can't, put this creature into its owner's graveyard instead of onto the battlefield. For each creature card exiled this way, this creature enters with a +2/+0, +1/+1, or +0/+2 counter on it.
//! Set: DRK #45 — The Dark | Scryfall ID: 8f99894d-5ece-44f1-acce-474494ae2084 | Oracle ID: 100218ce-1572-4bce-bb00-d2df5f68705c

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FRANKENSTEIN_S_MONSTER,
    oracle_id = "100218ce-1572-4bce-bb00-d2df5f68705c",
    scryfall_id = "8f99894d-5ece-44f1-acce-474494ae2084",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Frankenstein's Monster",
        mana_cost = mana!("{X}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(0),
        toughness = Some(1),
        enter_modifiers = &[
            EnterModifier::ExileFromGraveyardXOrGraveyard(&Filter::CREATURE),
            EnterModifier::ChooseCounterPerEntryCount(&[
                CounterKind::Plus {
                    power: 2,
                    toughness: 0,
                },
                CounterKind::P1P1,
                CounterKind::Plus {
                    power: 0,
                    toughness: 2,
                },
            ]),
        ],
    ),],
);
