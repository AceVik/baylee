//! Grim Feast — {1}{B}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, this enchantment deals 1 damage to you.
//! Oracle: Whenever a creature is put into an opponent's graveyard from the battlefield, you gain life equal to its toughness.
//! Set: MIR #265 — Mirage | Scryfall ID: a69dc4ac-7354-465e-b859-d8556f3b1498 | Oracle ID: 331233ba-0ddc-4cb2-b3b7-1b782e03529f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static OPPONENT_OWNED_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::OwnedByYou)]);

card!(
    index = index::GRIM_FEAST,
    oracle_id = "331233ba-0ddc-4cb2-b3b7-1b782e03529f",
    scryfall_id = "a69dc4ac-7354-465e-b859-d8556f3b1498",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Grim Feast",
        mana_cost = mana!("{1}{B}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::You),
            }],
        ),
        triggered!(
            Trigger::Dies(&OPPONENT_OWNED_CREATURE),
            &[Effect::GainLife {
                amount: Amount::EventLastToughness,
            }],
        ),
    ],
);
