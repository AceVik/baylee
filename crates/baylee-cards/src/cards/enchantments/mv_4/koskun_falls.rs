//! Koskun Falls — {2}{B}{B} — World Enchantment
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you tap an untapped creature you control.
//! Oracle: Creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you.
//! Set: HML #55 — Homelands | Scryfall ID: 04292a4e-8910-4911-a76d-4f2c3e15da33 | Oracle ID: a9b9e798-debc-4f2c-9770-e16f670082f7
// PARTIAL — only the upkeep sentence is played: the attack tax is left off. "Creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you" needs a tax that applies only to attacks on you, and `CantAttackUnlessPayMana` taxes every attack by the creatures it names.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KOSKUN_FALLS,
    oracle_id = "a9b9e798-debc-4f2c-9770-e16f670082f7",
    scryfall_id = "04292a4e-8910-4911-a76d-4f2c3e15da33",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "the attack tax is not played: \"creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you\" needs a mana tax on attacks at you alone, and `CantAttackUnlessPayMana` taxes every attack by the creatures it names, planeswalkers and other players included; only the upkeep tap-or-sacrifice is played"
    ),
    faces = &[face!(
        name = "Koskun Falls",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::PlayerMayPayCostOr {
            player: PlayerRel::You,
            cost: &CostPart::TapOther(&Filter::YOUR_CREATURE),
            effect: &Effect::SacrificeSelf,
        }],
    )],
    // NOT SUPPORTED: "Creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you." `Modifier::CantAttackUnlessPayMana(2)` on the opponents' creatures taxes an attack at a planeswalker you control and, at a table of more than two, an attack at another player, which the card does not print. `AttackTaxLife` knows the defender; no mana tax does.
);
