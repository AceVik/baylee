//! Lat-Nam's Legacy — {1}{U} — Instant
//! Oracle: Shuffle a card from your hand into your library. If you do, draw two cards at the beginning of the next turn's upkeep.
//! Set: ME2 #52 — Masters Edition II | Scryfall ID: ce13ff7b-a03c-488f-a140-2c03b64fe218 | Oracle ID: f885ee54-3b26-4d61-9783-3930b0e625aa
// NOT SUPPORTED: mandatory hand-to-library shuffle when your hand is non-empty is approximated using BottomCardFromHand (which offers an optional min: 0 choice) followed by ShuffleLibrary.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAT_NAM_S_LEGACY,
    oracle_id = "f885ee54-3b26-4d61-9783-3930b0e625aa",
    scryfall_id = "ce13ff7b-a03c-488f-a140-2c03b64fe218",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "uses BottomCardFromHand (which offers an optional min: 0 choice instead of mandatory min: 1 when your hand is non-empty) followed by ShuffleLibrary; if you choose a card from your hand, shuffles it into your library and draws two cards at the beginning of the next turn's upkeep (and does nothing if your hand is empty)",
    ),
    faces = &[face!(
        name = "Lat-Nam's Legacy",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::BottomCardFromHand {
        player: PlayerRel::You,
        filter: &Filter::Any,
        then: &[
            Effect::ShuffleLibrary {
                who: PlayerRel::You,
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(2)],
            },
        ],
    }])],
);
