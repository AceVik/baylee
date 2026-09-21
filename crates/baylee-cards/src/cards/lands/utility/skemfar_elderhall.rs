//! Skemfar Elderhall — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: {2}{B}{B}{G}, {T}, Sacrifice this land: Up to one target creature you don't control gets -2/-2 until end of turn. Create two 1/1 green Elf Warrior creature tokens. Activate only as a sorcery.
//! Set: KHM #268 — Kaldheim | Scryfall ID: 82c2a0f7-0f53-4627-8be8-227fde331a69 | Oracle ID: 70965b80-c8ad-4718-ae20-12a4d8228898
// PARTIAL — enters tapped, {T}: Add {G}, and the two Elf Warriors the third
// ability makes; its -2/-2 half is dropped because it targets "up to one",
// which an activated ability cannot state.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

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
    coverage = Coverage::Partial(
        "the printed ability targets \"up to one\" creature, and AbilityDef::Activated \
         carries a bare TargetSpec — exactly one — so the -2/-2 half is dropped"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        // NOT SUPPORTED: "Up to one target creature you don't control gets
        // -2/-2 until end of turn." `AbilityDef::Activated::target` is an
        // `Option<TargetSpec>` and a bare spec is read as exactly one
        // target, so writing it here would be an ability that cannot be
        // activated with nothing else on the board — where the card only
        // asks for *up to* one. `TargetReq` (and with it
        // `TargetReq::up_to_one`) exists on spells, triggered abilities and
        // loyalty abilities, and has no door into an activated one.
        activated!(
            cost!("{2}{B}{B}{G}", TapSelf, SacrificeSelf),
            &[Effect::CreateTokenN {
                token: &generated_tokens::ELF_WARRIOR_1_1_GREEN,
                amount: Amount::Fixed(2),
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
