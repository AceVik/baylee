//! Dystopia — {1}{B}{B} — Enchantment
//! Oracle: Cumulative upkeep—Pay 1 life. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: At the beginning of each player's upkeep, that player sacrifices a green or white permanent of their choice.
//! Set: ME2 #88 — Masters Edition II | Scryfall ID: 452c1fe9-1640-4d0e-9f54-7c673b13b091 | Oracle ID: d0e6203b-cef2-470f-8c79-7b139290e2d5

use baylee_cards_dsl::prelude::*;

static GREEN_OR_WHITE: Filter =
    Filter::HasColor(ColorSet::from_slice(&[Color::Green, Color::White]));

card!(
    index = index::DYSTOPIA,
    oracle_id = "d0e6203b-cef2-470f-8c79-7b139290e2d5",
    scryfall_id = "452c1fe9-1640-4d0e-9f54-7c673b13b091",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dystopia",
        mana_cost = mana!("{1}{B}{B}"),
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
                Effect::PlayerMayPayLifeOr {
                    player: PlayerRel::You,
                    life: Amount::CountersOnSource(counters::AGE),
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
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::SacrificeFilter {
                who: PlayerRel::ActivePlayer,
                filter: &GREEN_OR_WHITE,
            }],
        ),
    ],
);
