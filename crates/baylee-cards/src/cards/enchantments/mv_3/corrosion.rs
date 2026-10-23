//! Corrosion — {1}{B}{R} — Enchantment
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: At the beginning of your upkeep, put a rust counter on each artifact target opponent controls. Then destroy each artifact with mana value less than or equal to the number of rust counters on it. Artifacts destroyed this way can't be regenerated.
//! Oracle: When this enchantment leaves the battlefield, remove all rust counters from all permanents.
//! Set: VIS #128 — Visions | Scryfall ID: 176122b2-f60f-4150-8c0c-757c8f8914d2 | Oracle ID: 3d3fd8a2-78d4-4f52-8fcd-0c6fa93f0043
// PARTIAL — cumulative upkeep {1}; at the beginning of your upkeep each artifact each opponent controls gets a rust counter, then every artifact with mana value at most its rust counters (up to 12) is destroyed and can't be regenerated; not played: "target opponent" (it is every opponent, the same in a duel) and removing all rust counters from all permanents when this leaves the battlefield (no effect removes a kind of counter from every permanent), so counters stay on the board afterwards.

use baylee_cards_dsl::counters;
use baylee_cards_dsl::prelude::*;

static OPPONENT_ARTIFACT: Filter = Filter::And(&[Filter::ARTIFACT, Filter::ControlledByOpponent]);

/// "Each artifact with mana value less than or equal to the number of rust
/// counters on it": an artifact of mana value M with at least M rust counters
/// is matched by the arm for M (counters at least M, mana value at most M), so
/// the arms together say "mana value at most its rust counters" for every mana
/// value up to 12 (twelve upkeeps of Corrosion cost more than any game pays).
static RUSTED_OUT: Filter = Filter::Or(&[
    Filter::And(&[
        Filter::ARTIFACT,
        Filter::CountersAtLeast(counters::RUST, 0),
        Filter::CmcAtMost(0),
    ]),
    Filter::And(&[
        Filter::ARTIFACT,
        Filter::CountersAtLeast(counters::RUST, 1),
        Filter::CmcAtMost(1),
    ]),
    Filter::And(&[
        Filter::ARTIFACT,
        Filter::CountersAtLeast(counters::RUST, 2),
        Filter::CmcAtMost(2),
    ]),
    Filter::And(&[
        Filter::ARTIFACT,
        Filter::CountersAtLeast(counters::RUST, 3),
        Filter::CmcAtMost(3),
    ]),
    Filter::And(&[
        Filter::ARTIFACT,
        Filter::CountersAtLeast(counters::RUST, 4),
        Filter::CmcAtMost(4),
    ]),
    Filter::And(&[
        Filter::ARTIFACT,
        Filter::CountersAtLeast(counters::RUST, 5),
        Filter::CmcAtMost(5),
    ]),
    Filter::And(&[
        Filter::ARTIFACT,
        Filter::CountersAtLeast(counters::RUST, 6),
        Filter::CmcAtMost(6),
    ]),
    Filter::And(&[
        Filter::ARTIFACT,
        Filter::CountersAtLeast(counters::RUST, 7),
        Filter::CmcAtMost(7),
    ]),
    Filter::And(&[
        Filter::ARTIFACT,
        Filter::CountersAtLeast(counters::RUST, 8),
        Filter::CmcAtMost(8),
    ]),
    Filter::And(&[
        Filter::ARTIFACT,
        Filter::CountersAtLeast(counters::RUST, 9),
        Filter::CmcAtMost(9),
    ]),
    Filter::And(&[
        Filter::ARTIFACT,
        Filter::CountersAtLeast(counters::RUST, 10),
        Filter::CmcAtMost(10),
    ]),
    Filter::And(&[
        Filter::ARTIFACT,
        Filter::CountersAtLeast(counters::RUST, 11),
        Filter::CmcAtMost(11),
    ]),
    Filter::And(&[
        Filter::ARTIFACT,
        Filter::CountersAtLeast(counters::RUST, 12),
        Filter::CmcAtMost(12),
    ]),
]);

card!(
    index = index::CORROSION,
    oracle_id = "3d3fd8a2-78d4-4f52-8fcd-0c6fa93f0043",
    scryfall_id = "176122b2-f60f-4150-8c0c-757c8f8914d2",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Partial(
        "cumulative upkeep {1}; the upkeep rust counter goes on each artifact each opponent controls (the printed target opponent is the only opponent in a duel) and artifacts with mana value at most their rust counters (up to 12) are destroyed and can't be regenerated; removing all rust counters from all permanents when this leaves the battlefield is not played, since no effect removes a kind of counter from every permanent, so the counters stay on the board afterwards",
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
            &[
                Effect::AddCounterFilter {
                    filter: &OPPONENT_ARTIFACT,
                    kind: counters::RUST,
                    amount: Amount::Fixed(1),
                },
                Effect::DestroyAll {
                    filter: &RUSTED_OUT,
                    no_regen: true,
                },
            ],
        ),
    ],
);
