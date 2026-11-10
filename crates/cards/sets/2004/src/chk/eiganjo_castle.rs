//! Eiganjo Castle — (no cost) — Legendary Land
//! Oracle: {T}: Add {W}.
//! Oracle: {W}, {T}: Prevent the next 2 damage that would be dealt to target legendary creature this turn.
//! Set: CHK #275 — Champions of Kamigawa | Scryfall ID: 219c1d76-40cf-4edf-8145-e6cec8ca39ad | Oracle ID: 895a0e00-20a9-44f8-9215-66edcdf016b7
// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EIGANJO_CASTLE,
    oracle_id = "895a0e00-20a9-44f8-9215-66edcdf016b7",
    scryfall_id = "219c1d76-40cf-4edf-8145-e6cec8ca39ad",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Eiganjo Castle",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        activated!(
            cost!("{W}", TapSelf),
            &[Effect::PreventNextDamage {
                target: TargetSpec::Object(&Filter::LEGENDARY_CREATURE),
                amount: Amount::Fixed(2),
            }],
            target = Some(TargetSpec::Object(&Filter::LEGENDARY_CREATURE))
        ),
    ],
);
