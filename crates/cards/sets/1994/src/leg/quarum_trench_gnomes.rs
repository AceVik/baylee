//! Quarum Trench Gnomes — {3}{R} — Creature — Gnome
//! Oracle: {T}: If target Plains is tapped for mana, it produces colorless mana instead of white mana. (This effect lasts indefinitely.)
//! Set: LEG #162 — Legends | Scryfall ID: 1c3b33bf-3074-406e-86f3-2a9843cf4862 | Oracle ID: ce5e52c9-4cb1-42aa-8403-bcd143d68704
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::QUARUM_TRENCH_GNOMES,
    oracle_id = "ce5e52c9-4cb1-42aa-8403-bcd143d68704",
    scryfall_id = "1c3b33bf-3074-406e-86f3-2a9843cf4862",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Quarum Trench Gnomes",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GNOME],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::ModifyTargets {
            modifier: Modifier::ReplaceTappedMana {
                from: ManaColor::White,
                to: ManaColor::Colorless
            },
            duration: Duration::Indefinitely,
        }],
        target = Some(TargetSpec::Object(&Filter::HasSubtype(
            subtypes::land::PLAINS
        )))
    )],
);
