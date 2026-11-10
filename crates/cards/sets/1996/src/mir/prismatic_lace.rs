//! Prismatic Lace — {U} — Instant
//! Oracle: Target permanent becomes the color or colors of your choice. (This effect lasts indefinitely.)
//! Set: MIR #84 — Mirage | Scryfall ID: d1129585-6d59-4217-9404-747a100f1e8c | Oracle ID: d4a3c9a5-3c91-4135-a19d-98eb5bcf5e00
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRISMATIC_LACE,
    oracle_id = "d4a3c9a5-3c91-4135-a19d-98eb5bcf5e00",
    scryfall_id = "d1129585-6d59-4217-9404-747a100f1e8c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Prismatic Lace",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::ChooseColors {
            subject: ColorSubject::Target,
            max: 5,
            duration: Duration::Indefinitely,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
    )],
);
