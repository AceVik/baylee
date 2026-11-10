//! Price of Progress — {1}{R} — Instant
//! Oracle: Price of Progress deals damage to each player equal to twice the number of nonbasic lands that player controls.
//! Set: EMA #141 — Eternal Masters | Scryfall ID: b1342144-7a15-438b-a848-3196238a79e8 | Oracle ID: e9da499c-fa43-4e94-8395-5c030ff39502
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRICE_OF_PROGRESS,
    oracle_id = "e9da499c-fa43-4e94-8395-5c030ff39502",
    scryfall_id = "b1342144-7a15-438b-a848-3196238a79e8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Price of Progress",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::DealDamage {
        amount: Amount::Times {
            base: &Amount::ControlledCount {
                who: PlayerRel::Recipient,
                filter: &Filter::NONBASIC_LAND,
            },
            factor: 2,
        },
        target: TargetSpec::Player(PlayerRel::EachPlayer),
    }])],
);
