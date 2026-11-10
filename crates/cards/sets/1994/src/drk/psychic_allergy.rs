//! Psychic Allergy — {3}{U}{U} — Enchantment
//! Oracle: As this enchantment enters, choose a color.
//! Oracle: At the beginning of each opponent's upkeep, this enchantment deals X damage to that player, where X is the number of nontoken permanents of the chosen color they control.
//! Oracle: At the beginning of your upkeep, destroy this enchantment unless you sacrifice two Islands.
//! Set: DRK #34 — The Dark | Scryfall ID: fec3275e-4491-43a8-9f23-d7b48177c103 | Oracle ID: 58e0cc31-7f7c-4cea-9453-d377a039da36

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const ACTIVE_PLAYER_NONTOKEN_OF_CHOSEN_COLOR: Filter = Filter::And(&[
    Filter::ControlledByActivePlayer,
    Filter::Not(&Filter::IsToken),
    Filter::OfSourceChosenColor,
]);

const YOUR_ISLAND: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasSubtype(subtypes::land::ISLAND),
]);

card!(
    index = index::PSYCHIC_ALLERGY,
    oracle_id = "58e0cc31-7f7c-4cea-9453-d377a039da36",
    scryfall_id = "fec3275e-4491-43a8-9f23-d7b48177c103",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Psychic Allergy",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::ChooseColor],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::Opponent,
            },
            &[Effect::DealDamage {
                amount: Amount::CountOf {
                    filter: &ACTIVE_PLAYER_NONTOKEN_OF_CHOSEN_COLOR,
                    zone: ZoneSel::Battlefield,
                },
                target: TargetSpec::Player(PlayerRel::ActivePlayer),
            }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMaySacrificeNOr {
                player: PlayerRel::You,
                filter: &YOUR_ISLAND,
                count: 2,
                effect: &Effect::Destroy {
                    target: TargetSpec::ThisObject,
                    no_regen: false,
                },
            }],
        ),
    ],
);
