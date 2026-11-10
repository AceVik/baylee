//! Skemfar Elderhall — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: {2}{B}{B}{G}, {T}, Sacrifice this land: Up to one target creature you don't control gets -2/-2 until end of turn. Create two 1/1 green Elf Warrior creature tokens. Activate only as a sorcery.
//! Set: KHM #268 — Kaldheim | Scryfall ID: 82c2a0f7-0f53-4627-8be8-227fde331a69 | Oracle ID: 70965b80-c8ad-4718-ae20-12a4d8228898

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

/// "Creature you don't control", which is not the same as an opponent's
/// creature: a teammate's creature is one too.
static NOT_YOURS: Filter = Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::ControlledByYou)]);

card!(
    index = index::SKEMFAR_ELDERHALL,
    oracle_id = "70965b80-c8ad-4718-ae20-12a4d8228898",
    scryfall_id = "82c2a0f7-0f53-4627-8be8-227fde331a69",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Skemfar Elderhall",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        // "Up to one": with no creature to shrink, the Elves are still made
        // (CR 115.6).
        activated!(
            cost!("{2}{B}{B}{G}", TapSelf, SacrificeSelf),
            &[
                Effect::PumpTarget {
                    power: Amount::NegXFixed(2),
                    toughness: Amount::NegXFixed(2),
                    keywords: KeywordSet::EMPTY,
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::CreateTokenN {
                    token: &generated_tokens::ELF_WARRIOR_1_1_GREEN,
                    amount: Amount::Fixed(2),
                },
            ],
            targets = Some(TargetReq::up_to_one(TargetSpec::Object(&NOT_YOURS))),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
