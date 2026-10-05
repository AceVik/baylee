//! Wall of Putrid Flesh — {2}{B} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Protection from white
//! Oracle: Prevent all damage that would be dealt to this creature by enchanted creatures.
//! Set: LEG #127 — Legends | Scryfall ID: 07a17b74-a9c9-419a-8369-9ab4fec213f2 | Oracle ID: e31d4be7-cd24-4287-b8ca-66f8612731a6
// PARTIAL — defender and protection from white are written; the prevention
// from enchanted creatures is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_PUTRID_FLESH,
    oracle_id = "e31d4be7-cd24-4287-b8ca-66f8612731a6",
    scryfall_id = "07a17b74-a9c9-419a-8369-9ab4fec213f2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Wall of Putrid Flesh",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(2),
        toughness = Some(4),
    ),],
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Partial(
        "the prevention is not written: no static Modifier prevents damage \
         from a filtered source (`PreventDamageToIt` is combat-only and \
         unfiltered; `ProtectionFrom` also stops targeting and blocking), and \
         no `Filter` names an enchanted creature"
    ),
    // NOT SUPPORTED: "Prevent all damage that would be dealt to this creature
    // by enchanted creatures." — `Modifier::PreventDamageToIt` prevents combat
    // damage from every source and names no source filter, so it would also
    // stop an unenchanted creature's combat damage and still let an enchanted
    // creature's noncombat damage through; `Modifier::ProtectionFrom` can
    // prevent a class of sources' damage but also stops them targeting and
    // blocking this creature, which the card does not print, and no `Filter`
    // matches a creature that has an Aura attached (`Filter::IsAttached` is
    // about the creature itself being attached to something). No other static
    // `Modifier` reaches damage.
    abilities = &[static_ability!(
        Filter::This,
        Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::White])))
    )],
);
