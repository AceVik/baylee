//! Heat Wave — {2}{R} — Enchantment
//! Oracle: Cumulative upkeep {R} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Blue creatures can't block creatures you control.
//! Oracle: Nonblue creatures can't block creatures you control unless their controller pays 1 life for each blocking creature they control.
//! Set: VIS #83 — Visions | Scryfall ID: 42dd0810-4528-4a88-add8-923bb2057821 | Oracle ID: d8f08699-6210-41c3-bfa0-c4b3bcbbaf1c
// PARTIAL — Requiring a blocking player to pay 1 life for each nonblue blocking creature they control is not yet supported by Modifier; in play, Heat Wave has cumulative upkeep {R} and prevents blue creatures from blocking creatures you control.

use baylee_cards_dsl::prelude::*;

static BLUE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
]);

card!(
    index = index::HEAT_WAVE,
    oracle_id = "d8f08699-6210-41c3-bfa0-c4b3bcbbaf1c",
    scryfall_id = "42dd0810-4528-4a88-add8-923bb2057821",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Requiring a blocking player to pay 1 life for each nonblue blocking creature they control is not yet supported by Modifier; in play, Heat Wave has cumulative upkeep {R} and prevents blue creatures from blocking creatures you control.",
    ),
    faces = &[face!(
        name = "Heat Wave",
        mana_cost = mana!("{2}{R}"),
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
                    each: mana!("{R}"),
                    times: Amount::CountersOnSource(counters::AGE),
                    paid: &[],
                    unpaid: &[Effect::SacrificeSelf],
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield,
            ))),
        ),
        static_ability!(
            Filter::YOUR_CREATURE,
            Modifier::CantBeBlockedBy(&BLUE_CREATURE),
        ),
    ],
);
