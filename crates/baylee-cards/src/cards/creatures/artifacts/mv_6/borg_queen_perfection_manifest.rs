//! Borg Queen, Perfection Manifest — {4}{B}{B} — Legendary Artifact Creature — Borg Noble
//! Oracle: Artifact creatures you control get +2/+0.
//! Oracle: When Borg Queen enters, assimilate target creature card from an opponent's graveyard. (Put it onto the battlefield under your control with a +1/+1 counter. It's a Borg artifact creature and loses all other creature types.)
//! Set: TRC #197 — Star Trek Commander | Scryfall ID: cb07e5e4-154e-4ba5-85a4-ecc78f1555d2 | Oracle ID: f9b46a1a-474f-4fac-8d71-131c1720e4c0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BORG_QUEEN_PERFECTION_MANIFEST,
    oracle_id = "f9b46a1a-474f-4fac-8d71-131c1720e4c0",
    scryfall_id = "cb07e5e4-154e-4ba5-85a4-ecc78f1555d2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Borg Queen, Perfection Manifest",
        mana_cost = mana!("{4}{B}{B}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::BORG, subtypes::creature::NOBLE],
        power = Some(1),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
