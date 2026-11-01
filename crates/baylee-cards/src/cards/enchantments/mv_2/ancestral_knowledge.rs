//! Ancestral Knowledge — {1}{U} — Enchantment
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: When this enchantment enters, look at the top ten cards of your library, then exile any number of them and put the rest back on top of your library in any order.
//! Oracle: When this enchantment leaves the battlefield, shuffle your library.
//! Set: WTH #32 — Weatherlight | Scryfall ID: 05b90d72-00ac-4423-8cdf-e1471c6cd0ae | Oracle ID: 2786f210-ec64-46ea-9a7f-b03759fb1dae
// NOT SUPPORTED: exiling any number of the looked-at top ten cards during the entry trigger is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ANCESTRAL_KNOWLEDGE,
    oracle_id = "2786f210-ec64-46ea-9a7f-b03759fb1dae",
    scryfall_id = "05b90d72-00ac-4423-8cdf-e1471c6cd0ae",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "implements cumulative upkeep {1}, reordering the top ten cards of your library on entry, and shuffling your library when it leaves the battlefield; exiling any number of the looked-at cards during the entry trigger is not yet modeled",
    ),
    faces = &[face!(
        name = "Ancestral Knowledge",
        mana_cost = mana!("{1}{U}"),
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
                ZoneRef::Battlefield
            ))),
        ),
        triggered!(Trigger::ETB, &[Effect::ReorderTopLibrary { count: 10 }]),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::ShuffleLibrary {
                who: PlayerRel::You,
            }],
        ),
    ],
);
