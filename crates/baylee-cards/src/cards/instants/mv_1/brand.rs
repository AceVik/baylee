//! Brand — {R} — Instant
//! Oracle: Gain control of all permanents you own. (This effect lasts indefinitely.)
//! Oracle: Cycling {2} ({2}, Discard this card: Draw a card.)
//! Set: USG #176 — Urza's Saga | Scryfall ID: bafbd034-c8a0-4798-a680-555c13bdd251 | Oracle ID: eec7500b-418d-4387-beb2-d53e047c1d22
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BRAND,
    oracle_id = "eec7500b-418d-4387-beb2-d53e047c1d22",
    scryfall_id = "bafbd034-c8a0-4798-a680-555c13bdd251",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Brand",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        spell!(&[Effect::GainControlAll {
            filter: &Filter::OwnedByYou,
            to: PlayerRel::You,
        }]),
        activated!(
            cost!("{2}", DiscardSelf),
            &[Effect::draw(1)],
            zone = ActivationZone::Hand,
        ),
    ],
);
