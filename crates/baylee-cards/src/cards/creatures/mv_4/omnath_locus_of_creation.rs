//! Omnath, Locus of Creation — {R}{G}{W}{U} — Legendary Creature — Elemental
//! Oracle: When Omnath enters, draw a card.
//! Oracle: Landfall — Whenever a land you control enters, you gain 4 life if this is the first time this ability has resolved this turn. If it's the second time, add {R}{G}{W}{U}. If it's the third time, Omnath deals 4 damage to each opponent and each planeswalker you don't control.
//! Set: ZNR #232 — Zendikar Rising | Scryfall ID: 4e4fb50c-a81f-44d3-93c5-fa9a0b37f617 | Oracle ID: cd133d30-51ff-4114-a7d7-029345f0f0d7
// IMPLEMENTED — draws as it enters; landfall gains 4 life, then adds
// {R}{G}{W}{U}, then deals 4 to each opponent and each planeswalker you
// don't control, by how often it has resolved this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OMNATH_LOCUS_OF_CREATION,
    oracle_id = "cd133d30-51ff-4114-a7d7-029345f0f0d7",
    scryfall_id = "4e4fb50c-a81f-44d3-93c5-fa9a0b37f617",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Omnath, Locus of Creation",
        mana_cost = mana!("{R}{G}{W}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(4),
        toughness = Some(4),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        // "When Omnath enters, draw a card."
        triggered!(Trigger::ETB, &[Effect::draw(1)]),
        // "Landfall — Whenever a land you control enters, you gain 4 life if
        // this is the first time this ability has resolved this turn. If
        // it's the second time, add {R}{G}{W}{U}. If it's the third time,
        // Omnath deals 4 damage to each opponent and each planeswalker you
        // don't control."
        triggered!(
            Trigger::EntersBattlefield(&Filter::YOUR_LAND),
            &[Effect::NthResolutionThisTurn {
                effects: &[
                    Effect::gain_life(4),
                    Effect::Sequence(&[
                        Effect::mana(ManaColor::Red, 1),
                        Effect::mana(ManaColor::Green, 1),
                        Effect::mana(ManaColor::White, 1),
                        Effect::mana(ManaColor::Blue, 1),
                    ]),
                    Effect::Sequence(&[
                        Effect::DealDamage {
                            amount: Amount::Fixed(4),
                            target: TargetSpec::Player(PlayerRel::EachOpponent),
                        },
                        Effect::DealDamageEach {
                            amount: Amount::Fixed(4),
                            filter: &Filter::And(&[
                                Filter::PLANESWALKER,
                                Filter::Not(&Filter::ControlledByYou),
                            ]),
                        },
                    ]),
                ],
            }],
        ),
    ],
);
