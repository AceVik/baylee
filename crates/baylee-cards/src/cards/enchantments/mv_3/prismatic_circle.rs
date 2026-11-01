//! Prismatic Circle — {2}{W} — Enchantment
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: As this enchantment enters, choose a color.
//! Oracle: {1}: The next time a source of your choice of the chosen color would deal damage to you this turn, prevent that damage.
//! Set: MIR #34 — Mirage | Scryfall ID: 8d4e3de4-f173-4bcc-a5fb-089eee7108d3 | Oracle ID: 022dae2e-7fc3-486e-9168-59652d9ab21c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRISMATIC_CIRCLE,
    oracle_id = "022dae2e-7fc3-486e-9168-59652d9ab21c",
    scryfall_id = "8d4e3de4-f173-4bcc-a5fb-089eee7108d3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Prismatic Circle",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::ChooseColor],
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
        activated!(
            cost!("{1}"),
            &[Effect::PreventNextFromChosenSource {
                sources: &Filter::OfSourceChosenColor,
                combat_only: false,
                all_but: 0,
                gain_life: false,
            }],
        ),
    ],
);
