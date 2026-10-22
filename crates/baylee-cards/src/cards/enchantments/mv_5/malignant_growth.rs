//! Malignant Growth — {3}{G}{U} — Enchantment
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: At the beginning of your upkeep, put a growth counter on this enchantment.
//! Oracle: At the beginning of each opponent's draw step, that player draws an additional card for each growth counter on this enchantment, then this enchantment deals damage to the player equal to the number of cards they drew this way.
//! Set: MIR #272 — Mirage | Scryfall ID: 760b6703-ac92-45f6-8c32-60f760eba866 | Oracle ID: b3235d21-ce52-4a99-a2fe-99b0fe7cc8bc
// NOT SUPPORTED: tracking the exact number of cards drawn this way when a draw is replaced is not in ThisWay.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MALIGNANT_GROWTH,
    oracle_id = "b3235d21-ce52-4a99-a2fe-99b0fe7cc8bc",
    scryfall_id = "760b6703-ac92-45f6-8c32-60f760eba866",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    coverage = Coverage::Partial(
        "has cumulative upkeep {1}, puts a growth counter on itself at the beginning of your upkeep, and at the beginning of each opponent's draw step has that player draw a card and deals 1 damage to them for each growth counter on it; tracking the exact number of cards drawn this way when a draw is replaced is not in ThisWay",
    ),
    faces = &[face!(
        name = "Malignant Growth",
        mana_cost = mana!("{3}{G}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounter {
                    kind: counters::AGE,
                    amount: Amount::Fixed(1),
                },
                Effect::PlayerMayPayOr {
                    player: PlayerRel::You,
                    mana: Amount::CountersOnSource(counters::AGE),
                    effect: &Effect::SacrificeSelf,
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield,
            ))),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::AddCounter {
                kind: counters::GROWTH,
                amount: Amount::Fixed(1),
            }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Draw,
                whose: PlayerRel::EachOpponent,
            },
            &[
                Effect::DrawCardsFor {
                    amount: Amount::CountersOnSource(counters::GROWTH),
                    who: PlayerRel::ActivePlayer,
                },
                Effect::DealDamage {
                    amount: Amount::CountersOnSource(counters::GROWTH),
                    target: TargetSpec::Player(PlayerRel::ActivePlayer),
                },
            ],
        ),
    ],
);
