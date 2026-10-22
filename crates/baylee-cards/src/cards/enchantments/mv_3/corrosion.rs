//! Corrosion — {1}{B}{R} — Enchantment
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: At the beginning of your upkeep, put a rust counter on each artifact target opponent controls. Then destroy each artifact with mana value less than or equal to the number of rust counters on it. Artifacts destroyed this way can't be regenerated.
//! Oracle: When this enchantment leaves the battlefield, remove all rust counters from all permanents.
//! Set: VIS #128 — Visions | Scryfall ID: 176122b2-f60f-4150-8c0c-757c8f8914d2 | Oracle ID: 3d3fd8a2-78d4-4f52-8fcd-0c6fa93f0043
// PARTIAL — has cumulative upkeep {1} and puts a rust counter on each artifact opponents control at the beginning of your upkeep; no targeted-opponent filter, mana-value-to-rust-counter destruction, or LTB removal of all rust counters from all permanents exists yet.

use baylee_cards_dsl::counters;
use baylee_cards_dsl::prelude::*;

static OPPONENT_ARTIFACT: Filter = Filter::And(&[Filter::ARTIFACT, Filter::ControlledByOpponent]);

card!(
    index = index::CORROSION,
    oracle_id = "3d3fd8a2-78d4-4f52-8fcd-0c6fa93f0043",
    scryfall_id = "176122b2-f60f-4150-8c0c-757c8f8914d2",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Partial(
        "has cumulative upkeep {1} and puts a rust counter on each artifact opponents control at the beginning of your upkeep; no targeted-opponent filter, mana-value-to-rust-counter destruction, or LTB removal of all rust counters from all permanents exists yet",
    ),
    faces = &[face!(
        name = "Corrosion",
        mana_cost = mana!("{1}{B}{R}"),
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
                Effect::PlayerMayPayManaEachThenElse {
                    player: PlayerRel::You,
                    each: mana!("{1}"),
                    times: Amount::CountersOnSource(counters::AGE),
                    paid: &[],
                    unpaid: &[Effect::SacrificeSelf],
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
            &[Effect::AddCounterFilter {
                filter: &OPPONENT_ARTIFACT,
                kind: counters::RUST,
                amount: Amount::Fixed(1),
            }],
        ),
    ],
);
