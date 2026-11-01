//! Rune of Protection: Artifacts — {1}{W} — Enchantment
//! Oracle: {W}: The next time an artifact source of your choice would deal damage to you this turn, prevent that damage.
//! Oracle: Cycling {2} ({2}, Discard this card: Draw a card.)
//! Set: USG #35 — Urza's Saga | Scryfall ID: e18cce33-92be-4189-9f99-cb47bd617fd2 | Oracle ID: 469c4b77-733e-472b-aeae-6015b409beb2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RUNE_OF_PROTECTION_ARTIFACTS,
    oracle_id = "469c4b77-733e-472b-aeae-6015b409beb2",
    scryfall_id = "e18cce33-92be-4189-9f99-cb47bd617fd2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rune of Protection: Artifacts",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        activated!(
            cost!("{W}"),
            &[Effect::PreventNextFromChosenSource {
                sources: &Filter::ARTIFACT,
                combat_only: false,
                all_but: 0,
                gain_life: false,
            }],
        ),
        activated!(
            cost!("{2}", DiscardSelf),
            &[Effect::draw(1)],
            zone = ActivationZone::Hand,
        ),
    ],
);
