//! Gamekeeper — {3}{G} — Creature — Elf
//! Oracle: When this creature dies, you may exile it. If you do, reveal cards from the top of your library until you reveal a creature card. Put that card onto the battlefield and put all other cards revealed this way into your graveyard.
//! Set: DMR #162 — Dominaria Remastered | Scryfall ID: 1dba14c0-18c0-4ded-ae56-5a689106f1a1 | Oracle ID: c6921cfe-48f9-4378-b145-d9b430c2aaf6
// PARTIAL: when this creature dies, you may exile it and reveal cards from the top of your library until you reveal a creature card, putting that card onto the battlefield; RevealUntil puts the other revealed cards on the bottom of your library in a random order rather than into your graveyard.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GAMEKEEPER,
    oracle_id = "c6921cfe-48f9-4378-b145-d9b430c2aaf6",
    scryfall_id = "1dba14c0-18c0-4ded-ae56-5a689106f1a1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "when this creature dies, you may exile it and reveal cards from the top of your library until you reveal a creature card, putting that card onto the battlefield; RevealUntil puts the other revealed cards on the bottom of your library in a random order rather than into your graveyard",
    ),
    faces = &[face!(
        name = "Gamekeeper",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::MayDo {
            effects: &[
                Effect::exile(TargetSpec::ThisObject),
                Effect::RevealUntil {
                    filter: &Filter::CREATURE,
                    found: SearchDest::Battlefield,
                },
            ],
        }],
    )],
);
