//! Mind Maggots — {3}{B} — Creature — Insect
//! Oracle: When this creature enters, discard any number of creature cards. For each card discarded this way, put two +1/+1 counters on this creature.
//! Set: EXO #66 — Exodus | Scryfall ID: c3c92a7f-a250-4497-aa7a-0394e94ef13d | Oracle ID: fcaa5a54-eb73-4d78-96a3-826d1902f642
// PARTIAL — Discarding any number of creature cards on entry is not supported; when this creature enters, you may discard a creature card to put two +1/+1 counters on it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: Discarding any number of creature cards on entry is not supported; when this creature enters, you may discard a creature card to put two +1/+1 counters on it.
card!(
    index = index::MIND_MAGGOTS,
    oracle_id = "fcaa5a54-eb73-4d78-96a3-826d1902f642",
    scryfall_id = "c3c92a7f-a250-4497-aa7a-0394e94ef13d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Discarding any number of creature cards on entry is not supported; when this creature enters, you may discard a creature card to put two +1/+1 counters on it."
    ),
    faces = &[face!(
        name = "Mind Maggots",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::MayDo {
            effects: &[
                Effect::DiscardMatching {
                    who: PlayerRel::You,
                    filter: &Filter::CREATURE,
                },
                Effect::AddCounterSource {
                    kind: CounterKind::P1P1,
                    amount: Amount::Times {
                        base: &Amount::ThisWay(ThisWay::Discarded),
                        factor: 2,
                    },
                },
            ],
        }],
    )],
);
