//! Shadowbane — {1}{W} — Instant
//! Oracle: The next time a source of your choice would deal damage to you and/or creatures you control this turn, prevent that damage. If damage from a black source is prevented this way, you gain that much life.
//! Set: MIR #38 — Mirage | Scryfall ID: 0fbe8fa5-5ca8-43ce-863e-61802126e485 | Oracle ID: 9e9ec33b-b923-40e5-81a8-e1e78df63ec7
// PARTIAL: Effect::PreventNextFromChosenSource prevents the next damage the chosen source would deal to you rather than also protecting creatures you control, and does not check whether the source is black to grant life; prevents the next damage a chosen source would deal to you this turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHADOWBANE,
    oracle_id = "9e9ec33b-b923-40e5-81a8-e1e78df63ec7",
    scryfall_id = "0fbe8fa5-5ca8-43ce-863e-61802126e485",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Effect::PreventNextFromChosenSource prevents the next damage the chosen source would deal to you rather than also protecting creatures you control, and does not check whether the source is black to grant life; prevents the next damage a chosen source would deal to you this turn",
    ),
    faces = &[face!(
        name = "Shadowbane",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::PreventNextFromChosenSource {
        sources: &Filter::Any,
        combat_only: false,
        all_but: 0,
        gain_life: false,
    }])],
);
