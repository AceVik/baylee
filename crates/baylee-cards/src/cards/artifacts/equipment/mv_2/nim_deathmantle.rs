//! Nim Deathmantle — {2} — Artifact — Equipment
//! Oracle: Equipped creature gets +2/+2, has intimidate, and is a black Zombie. (A creature with intimidate can't be blocked except by artifact creatures and/or creatures that share a color with it.)
//! Oracle: Whenever a nontoken creature is put into your graveyard from the battlefield, you may pay {4}. If you do, return that card to the battlefield and attach this Equipment to it.
//! Oracle: Equip {4}
//! Set: 2X2 #309 — Double Masters 2022 | Scryfall ID: 787b1cc8-42b4-4d3e-9b8a-a252de297b1a | Oracle ID: 66d41377-626d-4ae6-ba86-17bf0c8b3362
// PARTIAL — the equip half: +2/+2, black, Zombie, equip {4}. The {4}
// reanimation trigger and intimidate are not expressible in this DSL.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NIM_DEATHMANTLE,
    oracle_id = "66d41377-626d-4ae6-ba86-17bf0c8b3362",
    scryfall_id = "787b1cc8-42b4-4d3e-9b8a-a252de297b1a",
    faces = &[face!(
        name = "Nim Deathmantle",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
    coverage = Coverage::Partial(
        "the '{4}, if you do' reanimation has no variant whose effect runs when \
         the mana is paid — Effect::PlayerMayPayOr runs its effect on refusal, \
         which would hand the creature back for free — and intimidate is a \
         keyword bit no rule in the engine reads"
    ),
    abilities = &[
        // NOT SUPPORTED: "has intimidate" — intimidate is not one of the
        // keyword bits the engine reads (flying, deathtouch, menace, …), so
        // the clause of this same sentence has to be dropped rather than
        // granted through Modifier::AddKeyword.
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 2)),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddColor(ColorSet::from_slice(&[Color::Black]))
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddSubtype(subtypes::creature::ZOMBIE)
        ),
        equip!("{4}"),
        // NOT SUPPORTED: "Whenever a nontoken creature is put into your
        // graveyard from the battlefield, you may pay {4}. If you do, return
        // that card to the battlefield and attach this Equipment to it." —
        // the paying half has no effect. Effect::PlayerMayPayOr is the only
        // pay-or-else construct, and its `effect` runs when the player does
        // *not* pay; written that way this ability would reanimate the
        // creature for nothing, so it comes off the card instead.
    ],
);
