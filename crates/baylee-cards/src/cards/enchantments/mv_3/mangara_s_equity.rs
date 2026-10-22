//! Mangara's Equity — {1}{W}{W} — Enchantment
//! Oracle: As this enchantment enters, choose black or red.
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you pay {1}{W}.
//! Oracle: Whenever a creature of the chosen color deals damage to you or a white creature you control, this enchantment deals that much damage to that creature.
//! Set: MIR #26 — Mirage | Scryfall ID: 6d796153-3f7f-4d94-8ad4-5a4b2c8e09bb | Oracle ID: c7986014-30e0-4999-ab95-b86620471419
// NOT SUPPORTED: restricting the entry color choice to black or red and triggering when a creature of the chosen color deals damage to a white creature you control are not supported.

use baylee_cards_dsl::prelude::*;

const CREATURE_OF_CHOSEN_COLOR: Filter =
    Filter::And(&[Filter::CREATURE, Filter::OfSourceChosenColor]);

card!(
    index = index::MANGARA_S_EQUITY,
    oracle_id = "c7986014-30e0-4999-ab95-b86620471419",
    scryfall_id = "6d796153-3f7f-4d94-8ad4-5a4b2c8e09bb",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "has you choose a color as it enters, sacrifices itself at your upkeep unless you pay {1}{W}, and deals that much damage back to any creature of the chosen color that deals damage to you; restricting the color choice to black or red and triggering when damage is dealt to a white creature you control are not supported",
    ),
    faces = &[face!(
        name = "Mangara's Equity",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::ChooseColor],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayManaOr {
                player: PlayerRel::You,
                cost: mana!("{1}{W}"),
                effect: &Effect::SacrificeSelf,
            }],
        ),
        triggered!(
            Trigger::DealsDamageToPlayer {
                source: &CREATURE_OF_CHOSEN_COLOR,
                player: PlayerRel::You,
            },
            &[Effect::DealDamage {
                amount: Amount::EventAmount,
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::EventObject)),
        ),
    ],
);
