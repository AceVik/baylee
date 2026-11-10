//! Miraculous Recovery — {4}{W} — Instant
//! Oracle: Return target creature card from your graveyard to the battlefield. Put a +1/+1 counter on it.
//! Set: UMA #26 — Ultimate Masters | Scryfall ID: 2b3459d9-a667-4cee-9b39-844013576d0b | Oracle ID: cea73a4b-e10a-4517-a92b-24a08321009f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const YOUR_GRAVEYARD_CREATURE: TargetSpec =
    TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You);

card!(
    index = index::MIRACULOUS_RECOVERY,
    oracle_id = "cea73a4b-e10a-4517-a92b-24a08321009f",
    scryfall_id = "2b3459d9-a667-4cee-9b39-844013576d0b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Miraculous Recovery",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::GraveyardToBattlefield {
            target: YOUR_GRAVEYARD_CREATURE,
            owner_control: false,
            counters: Some((CounterKind::P1P1, 1)),
        }],
        targets = Some(TargetReq::one(YOUR_GRAVEYARD_CREATURE)),
    )],
);
