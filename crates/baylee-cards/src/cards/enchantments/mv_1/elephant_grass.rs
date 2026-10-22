//! Elephant Grass — {G} — Enchantment
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Black creatures can't attack you.
//! Oracle: Nonblack creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you.
//! Set: VIS #104 — Visions | Scryfall ID: f4c1f5a7-0d28-43ab-9b66-937e963f42cd | Oracle ID: 53cc4b93-538d-4193-8bbb-5b73d3ce9d79
// PARTIAL — has cumulative upkeep {1}, prevents black creatures from attacking you, and requires opponents' nonblack creatures to pay {2} to attack; no defender-specific mana attack tax ('attacking you' only, rather than any player or planeswalker) exists yet.

use baylee_cards_dsl::prelude::*;

static BLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
]);
static OPPONENT_NONBLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::ControlledByOpponent,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
]);

card!(
    index = index::ELEPHANT_GRASS,
    oracle_id = "53cc4b93-538d-4193-8bbb-5b73d3ce9d79",
    scryfall_id = "f4c1f5a7-0d28-43ab-9b66-937e963f42cd",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "has cumulative upkeep {1}, prevents black creatures from attacking you, and requires opponents' nonblack creatures to pay {2} to attack; no defender-specific mana attack tax ('attacking you' only, rather than any player or planeswalker) exists yet",
    ),
    faces = &[face!(
        name = "Elephant Grass",
        mana_cost = mana!("{G}"),
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
        static_ability!(BLACK_CREATURE, Modifier::CantAttackYou),
        static_ability!(
            OPPONENT_NONBLACK_CREATURE,
            Modifier::CantAttackUnlessPayMana(2)
        ),
    ],
);
