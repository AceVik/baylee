//! Argothian Pixies — {1}{G} — Creature — Faerie
//! Oracle: This creature can't be blocked by artifact creatures.
//! Oracle: Prevent all damage that would be dealt to this creature by artifact creatures.
//! Set: ME4 #142 — Masters Edition IV | Scryfall ID: 6d672009-a442-495a-b2aa-b25b92db7030 | Oracle ID: bbf183bc-d502-4432-8202-f29f60c08396
// PARTIAL — can't be blocked by artifact creatures is written; the prevention is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ARTIFACT_CREATURE: Filter = Filter::And(&[Filter::ARTIFACT, Filter::CREATURE]);

card!(
    index = index::ARGOTHIAN_PIXIES,
    oracle_id = "bbf183bc-d502-4432-8202-f29f60c08396",
    scryfall_id = "6d672009-a442-495a-b2aa-b25b92db7030",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Argothian Pixies",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FAERIE],
        power = Some(2),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "the prevention is not written: no static Modifier prevents damage \
         from a filtered source (PreventDamageToIt is combat-only and \
         unfiltered; ProtectionFrom also stops targeting and blocking)"
    ),
    // NOT SUPPORTED: "Prevent all damage that would be dealt to this creature
    // by artifact creatures." — `Modifier::PreventDamageToIt` prevents combat
    // damage from every source and names no source filter, so it would also
    // stop a nonartifact creature's combat damage and still let an artifact
    // creature's noncombat damage through; `Modifier::ProtectionFrom` can
    // name artifact creatures and does prevent their damage, but it also
    // stops them targeting and blocking this creature, which the card does
    // not print. No other static `Modifier` reaches damage.
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CantBeBlockedBy(&ARTIFACT_CREATURE)
    )],
);
