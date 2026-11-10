//! Elvish Healer — {2}{W} — Creature — Elf Cleric
//! Oracle: {T}: Prevent the next 1 damage that would be dealt to any target this turn. If it's a green creature, prevent the next 2 damage instead.
//! Set: ICE #22 — Ice Age | Scryfall ID: 00bd8485-d63a-4077-a3d1-4d0f2f4d8035 | Oracle ID: ea46835c-9dac-4e1e-8338-ad99136f511a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const GREEN_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
]);

card!(
    index = index::ELVISH_HEALER,
    oracle_id = "ea46835c-9dac-4e1e-8338-ad99136f511a",
    scryfall_id = "00bd8485-d63a-4077-a3d1-4d0f2f4d8035",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Elvish Healer",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!(TapSelf),
        &[Effect::IfTargetMatchesElse {
            filter: &GREEN_CREATURE,
            then: &[Effect::PreventNextDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(2),
            }],
            otherwise: &[Effect::PreventNextDamage {
                target: TargetSpec::AnyTarget,
                amount: Amount::Fixed(1),
            }],
        }],
        target = Some(TargetSpec::AnyTarget),
    )],
);
