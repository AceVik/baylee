//! Argothian Treefolk — {3}{G}{G} — Creature — Treefolk
//! Oracle: Prevent all damage that would be dealt to this creature by artifact sources.
//! Set: ME4 #143 — Masters Edition IV | Scryfall ID: 36b0d5b3-84d7-4888-90e9-2d0eb16c11d6 | Oracle ID: f3aaef18-dc32-40d6-b48c-f957aa31247f
// PARTIAL — the prevention is off the card: no static Modifier prevents damage from a filtered source.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARGOTHIAN_TREEFOLK,
    oracle_id = "f3aaef18-dc32-40d6-b48c-f957aa31247f",
    scryfall_id = "36b0d5b3-84d7-4888-90e9-2d0eb16c11d6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Argothian Treefolk",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TREEFOLK],
        power = Some(3),
        toughness = Some(5),
    ),],
    coverage = Coverage::Partial(
        "no static Modifier prevents damage from a filtered source: \
         Modifier::PreventDamageToIt is combat-only and takes no source \
         filter, and Modifier::ProtectionFrom also stops artifacts from \
         targeting and blocking this creature, which the card does not print"
    ),
    // NOT SUPPORTED: "Prevent all damage that would be dealt to this creature
    // by artifact sources." — the only prevention vocabulary is
    // `Modifier::PreventDamageToIt`, which prevents combat damage from every
    // source and names no source filter, so it would also stop a nonartifact
    // creature's combat damage and still let an artifact's noncombat damage
    // through; `Modifier::ProtectionFrom(&Filter::ARTIFACT)` does prevent all
    // of the artifact's damage but also stops artifacts from targeting and
    // blocking this creature, a sentence the card does not print. No other
    // static `Modifier` reaches damage.
    abilities = &[],
);
