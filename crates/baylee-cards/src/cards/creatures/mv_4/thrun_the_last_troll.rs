//! Thrun, the Last Troll — {2}{G}{G} — Legendary Creature — Troll Shaman
//! Oracle: This spell can't be countered.
//! Oracle: Hexproof (This creature can't be the target of spells or abilities your opponents control.)
//! Oracle: {1}{G}: Regenerate Thrun.
//! Set: MBS #92 — Mirrodin Besieged | Scryfall ID: 5d393da0-4cb6-4ae8-b747-8e6d0fa7f55a | Oracle ID: 1149e5ac-554a-41b1-84ae-bac42579c1aa
// PARTIAL — "can't be countered" and hexproof are the two keyword bits the
// engine reads, and both are stated here; the {1}{G} regenerate activation
// has no DSL variant and is dropped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THRUN_THE_LAST_TROLL,
    oracle_id = "1149e5ac-554a-41b1-84ae-bac42579c1aa",
    scryfall_id = "5d393da0-4cb6-4ae8-b747-8e6d0fa7f55a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Thrun, the Last Troll",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::TROLL, subtypes::creature::SHAMAN],
        power = Some(4),
        toughness = Some(4),
    ),],
    keywords = KeywordSet::HEXPROOF.union(KeywordSet::UNCOUNTERABLE),
    coverage = Coverage::Partial(
        "the {1}{G} regenerate activation is dropped: no Effect::Regenerate exists"
    ),
);

// NOT SUPPORTED: {1}{G}: Regenerate Thrun. — a regeneration shield is a
// replacement effect on destruction ("the next time it would be destroyed
// this turn, instead tap it, remove it from combat, and remove all damage"),
// and the effect vocabulary has no `Regenerate` and nothing that stands in
// for one. `Modifier::PreventDamageToIt` is a different sentence and would be
// a card that claims `Implemented` and lets the creature die to `Destroy`.
