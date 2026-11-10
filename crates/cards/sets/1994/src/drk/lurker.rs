//! Lurker — {2}{G} — Creature — Beast
//! Oracle: This creature can't be the target of spells unless it attacked or blocked this turn.
//! Set: DRK #80 — The Dark | Scryfall ID: b39eb671-e17e-4c5a-8913-1e3be7faedfb | Oracle ID: 4aedbbaf-b6ae-4bf0-84eb-31b0b28cfdfa

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static THIS_DIDNT_ATTACK_OR_BLOCK: Filter = Filter::And(&[
    Filter::This,
    Filter::Not(&Filter::AttackedThisTurn),
    Filter::Not(&Filter::BlockedThisTurn),
]);

static SPELL_ON_STACK: Filter = Filter::InZone(ZoneRef::Stack);

card!(
    index = index::LURKER,
    oracle_id = "4aedbbaf-b6ae-4bf0-84eb-31b0b28cfdfa",
    scryfall_id = "b39eb671-e17e-4c5a-8913-1e3be7faedfb",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lurker",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[static_ability!(
        THIS_DIDNT_ATTACK_OR_BLOCK,
        Modifier::CantBeTargetedBy(&SPELL_ON_STACK)
    )],
);
