//! Heartwood Dryad — {1}{G} — Creature — Dryad
//! Oracle: This creature can block creatures with shadow as though it had shadow.
//! Set: TPR #173 — Tempest Remastered | Scryfall ID: edeb7cc6-68a5-4825-ad1f-6afd6b0c0b87 | Oracle ID: 976968c5-7ef4-4451-86ff-03a011bbcca7
// PARTIAL — Only the 2/1 creature body is supported; as-though-shadow blocking without granting actual shadow is unsupported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HEARTWOOD_DRYAD,
    oracle_id = "976968c5-7ef4-4451-86ff-03a011bbcca7",
    scryfall_id = "edeb7cc6-68a5-4825-ad1f-6afd6b0c0b87",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Only the 2/1 creature body is supported; as-though-shadow blocking without granting actual shadow is unsupported."
    ),
    faces = &[face!(
        name = "Heartwood Dryad",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRYAD],
        power = Some(2),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: block shadow creatures as though it had shadow; actual shadow grants unprinted abilities.
    abilities = &[],
);
