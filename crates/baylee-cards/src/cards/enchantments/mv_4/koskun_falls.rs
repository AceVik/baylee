//! Koskun Falls — {2}{B}{B} — World Enchantment
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you tap an untapped creature you control.
//! Oracle: Creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you.
//! Set: HML #55 — Homelands | Scryfall ID: 04292a4e-8910-4911-a76d-4f2c3e15da33 | Oracle ID: a9b9e798-debc-4f2c-9770-e16f670082f7
// PARTIAL — CantAttackUnlessPayMana taxes all attacks by opposing creatures rather than only creatures attacking you directly; upkeep tap-a-creature-or-sacrifice and {2} attack tax on opponents' creatures work.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KOSKUN_FALLS,
    oracle_id = "a9b9e798-debc-4f2c-9770-e16f670082f7",
    scryfall_id = "04292a4e-8910-4911-a76d-4f2c3e15da33",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "CantAttackUnlessPayMana taxes all attacks by opposing creatures rather than only creatures attacking you directly; upkeep tap-a-creature-or-sacrifice and {2} attack tax on opponents' creatures work"
    ),
    faces = &[face!(
        name = "Koskun Falls",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayCostOr {
                player: PlayerRel::You,
                cost: &CostPart::TapOther(&Filter::YOUR_CREATURE),
                effect: &Effect::SacrificeSelf,
            }],
        ),
        static_ability!(
            Filter::OPPONENT_CREATURE,
            Modifier::CantAttackUnlessPayMana(2)
        ),
    ],
);
