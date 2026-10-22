//! Tornado — {4}{G} — Enchantment
//! Oracle: Cumulative upkeep {G} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: {2}{G}, Pay 3 life for each velocity counter on this enchantment: Destroy target permanent and put a velocity counter on this enchantment. Activate only once each turn.
//! Set: ME1 #136 — Masters Edition | Scryfall ID: 23f94cb6-1880-4c99-ba14-91807a6df92d | Oracle ID: dd283473-6423-4a1f-b5a3-77234cdce643

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TORNADO,
    oracle_id = "dd283473-6423-4a1f-b5a3-77234cdce643",
    scryfall_id = "23f94cb6-1880-4c99-ba14-91807a6df92d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Tornado",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Partial(
        "Cumulative upkeep {G} and the once-per-turn {2}{G} activation that destroys target permanent and puts a velocity counter on Tornado are implemented; paying 3 life for each velocity counter on Tornado as part of the activation cost is not implemented."
    ),
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
                    each: mana!("{G}"),
                    times: Amount::CountersOnSource(counters::AGE),
                    paid: &[],
                    unpaid: &[Effect::SacrificeSelf],
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield,
            ))),
        ),
        activated!(
            cost!("{2}{G}"),
            &[
                Effect::destroy(TargetSpec::Object(&Filter::Any)),
                Effect::AddCounterSource {
                    kind: counters::VELOCITY,
                    amount: Amount::Fixed(1),
                },
            ],
            target = Some(TargetSpec::Object(&Filter::Any)),
            limit = ActivationLimit::PerTurn(1)
        ),
    ],
);
