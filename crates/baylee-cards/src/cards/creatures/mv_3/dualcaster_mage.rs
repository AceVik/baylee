//! Dualcaster Mage — {1}{R}{R} — Creature — Human Wizard
//! Oracle: Flash
//! Oracle: When this creature enters, copy target instant or sorcery spell. You may choose new targets for the copy.
//! Set: C21 #165 — Commander 2021 | Scryfall ID: defcc4a3-40e0-4f5d-b23c-6cd6a614abc1 | Oracle ID: 8eb7c0a5-6190-40de-b473-2d1daa3bbe28
// IMPLEMENTED — flash, then an enter trigger that copies one instant or
// sorcery already on the stack. The copy is put there under your control and
// its targets are asked for again (CR 707.10c); re-picking what it already
// points at is how a player declines, so the "you may" needs no second answer.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DUALCASTER_MAGE,
    oracle_id = "8eb7c0a5-6190-40de-b473-2d1daa3bbe28",
    scryfall_id = "defcc4a3-40e0-4f5d-b23c-6cd6a614abc1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dualcaster Mage",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::CopyTargetSpell { mods: &[] }],
        targets = Some(TargetReq::one(TargetSpec::Spell(
            &Filter::INSTANT_OR_SORCERY
        )))
    )],
);

// Engine-level test belongs in baylee-engine (card_tests): every other card
// that reaches `CopyTargetSpell` from a trigger copies the spell that caused
// it (`TargetSpec::EventObject`), so this is the first one whose trigger picks
// a spell — the two paths worth playing once are flashing it in response to a
// burn spell and pointing the copy elsewhere, and an enter with an empty stack,
// where the trigger has no legal target and leaves the stack (CR 603.3d).
