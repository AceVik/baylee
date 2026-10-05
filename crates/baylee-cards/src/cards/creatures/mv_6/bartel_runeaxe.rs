//! Bartel Runeaxe — {3}{B}{R}{G} — Legendary Creature — Giant Warrior
//! Oracle: Vigilance
//! Oracle: Bartel Runeaxe can't be the target of Aura spells.
//! Set: ME3 #145 — Masters Edition III | Scryfall ID: 3c635e33-c9f1-4508-986d-c0289921c299 | Oracle ID: 9beccf09-c024-408b-9f84-fe2c7462babb
// PARTIAL — vigilance is written; the Aura-spell targeting restriction is off
// the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BARTEL_RUNEAXE,
    oracle_id = "9beccf09-c024-408b-9f84-fe2c7462babb",
    scryfall_id = "3c635e33-c9f1-4508-986d-c0289921c299",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Bartel Runeaxe",
        mana_cost = mana!("{3}{B}{R}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::GIANT, subtypes::creature::WARRIOR],
        power = Some(6),
        toughness = Some(5),
    ),],
    keywords = KeywordSet::VIGILANCE,
    coverage = Coverage::Partial(
        "\"can't be the target of Aura spells\" cannot be spelled: \
         `Modifier::CantBeTargetedBy` asks its filter of the spell or of the \
         ability's source, so a filter matching an Aura card also stops \
         abilities of Aura permanents, which the card does not",
    ),
    // NOT SUPPORTED: "Bartel Runeaxe can't be the target of Aura spells." —
    // `Modifier::CantBeTargetedBy(&Filter::HasSubtype(subtypes::enchantment::AURA))`
    // would also stop activated and triggered abilities whose source is an
    // Aura permanent, and no filter says "spell only" (at target announcement
    // a spell is still its owner's card, so `Filter::InZone(ZoneRef::Stack)`
    // does not match it either).
);
