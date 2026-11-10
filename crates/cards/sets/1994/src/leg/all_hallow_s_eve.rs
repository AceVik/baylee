//! All Hallow's Eve — {2}{B}{B} — Sorcery
//! Oracle: Exile All Hallow's Eve with two scream counters on it.
//! Oracle: At the beginning of your upkeep, if this card is exiled with a scream counter on it, remove a scream counter from it. If there are no more scream counters on it, put it into your graveyard and each player returns all creature cards from their graveyard to the battlefield.
//! Set: ME3 #57 — Masters Edition III | Scryfall ID: af266eec-e61b-4bda-be7f-3a5a59eee864 | Oracle ID: ebed900f-6de3-4ae7-9795-fae34882298d
use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALL_HALLOW_S_EVE,
    oracle_id = "ebed900f-6de3-4ae7-9795-fae34882298d",
    scryfall_id = "af266eec-e61b-4bda-be7f-3a5a59eee864",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "All Hallow's Eve",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        spell!(&[Effect::ExileSourceWithCounters {
            kind: counters::SCREAM,
            n: 2,
        }]),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::RemoveCounterSelf {
                    kind: counters::SCREAM,
                    n: 1,
                },
                Effect::IfNoCountersOnSelf {
                    kind: counters::SCREAM,
                    then: &[
                        Effect::MoveSourceToOwnerGraveyard,
                        Effect::EachPlayerGraveyardToBattlefield {
                            filter: &Filter::CREATURE,
                        },
                    ],
                },
            ],
            zone = TriggerZone::Exile,
            condition = Some(Condition::CountersOnSelf(counters::SCREAM, 1)),
        ),
    ],
);
