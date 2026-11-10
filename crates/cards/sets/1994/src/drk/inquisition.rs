//! Inquisition — {2}{B} — Sorcery
//! Oracle: Target player reveals their hand. Inquisition deals damage to that player equal to the number of white cards in their hand.
//! Set: DRK #47 — The Dark | Scryfall ID: 5f133f06-6398-4db1-8577-66c16fd3e00d | Oracle ID: 9178253e-0d89-450d-a93b-ed4765e945db

use baylee_cards_dsl::prelude::*;

static WHITE_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::White]));

card!(
    index = index::INQUISITION,
    oracle_id = "9178253e-0d89-450d-a93b-ed4765e945db",
    scryfall_id = "5f133f06-6398-4db1-8577-66c16fd3e00d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Inquisition",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::RevealHand {
                who: PlayerRel::Chosen,
            },
            Effect::DealDamage {
                amount: Amount::CountOf {
                    filter: &WHITE_CARD,
                    zone: ZoneSel::HandChosen,
                },
                target: TargetSpec::AnyPlayer,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);
