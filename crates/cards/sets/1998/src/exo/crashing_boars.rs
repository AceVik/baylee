//! Crashing Boars — {3}{G}{G} — Creature — Boar
//! Oracle: Whenever this creature attacks, defending player chooses an untapped creature they control. That creature blocks this creature this turn if able.
//! Set: TPR #168 — Tempest Remastered | Scryfall ID: e264adbc-1c52-4207-b2e6-0dd76a58f2d6 | Oracle ID: a4c59feb-e76a-4e08-b296-21c3e61cfc64
// PARTIAL — Having the defending player choose an untapped creature they control to force-block this creature when it attacks is not supported; enters as a vanilla 4/4.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: Having the defending player choose an untapped creature they control to force-block this creature when it attacks is not supported; enters as a vanilla 4/4.
card!(
    index = index::CRASHING_BOARS,
    oracle_id = "a4c59feb-e76a-4e08-b296-21c3e61cfc64",
    scryfall_id = "e264adbc-1c52-4207-b2e6-0dd76a58f2d6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Having the defending player choose an untapped creature they control to force-block this creature when it attacks is not supported; enters as a vanilla 4/4."
    ),
    faces = &[face!(
        name = "Crashing Boars",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BOAR],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[],
);
