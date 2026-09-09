//! Ephemerate — {W} — Instant
//! Oracle: Exile target creature you control, then return it to the battlefield under its owner's control.
//! Oracle: Rebound (If you cast this spell from your hand, exile it as it resolves. At the beginning of your next upkeep, you may cast this card from exile without paying its mana cost.)
//! Set: MH1 #7 — Modern Horizons | Scryfall ID: 2da5f3f8-5eef-498f-ba2c-2f3fbc3745aa | Oracle ID: 0fd57894-b917-41c8-a394-360d1d31b236
// IMPLEMENTED — blink + rebound (exile on resolution, free re-cast at
// your next upkeep).

static YOUR_CREATURE: Filter = Filter::And(&[Filter::ControlledByYou, Filter::CREATURE]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 44,
    oracle_id: "0fd57894-b917-41c8-a394-360d1d31b236",
    scryfall_id: "2da5f3f8-5eef-498f-ba2c-2f3fbc3745aa",
    faces: &[face! {
        name: "Ephemerate",
        mana_cost: baylee_core::mana!("{W}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::REBOUND,
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::Blink {
            target: TargetSpec::Object(&YOUR_CREATURE),
        }], targets: Some(TargetReq::one(TargetSpec::Object(&YOUR_CREATURE))))],
}
