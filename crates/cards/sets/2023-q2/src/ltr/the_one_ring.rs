//! The One Ring — {4} — Legendary Artifact
//! Oracle: Indestructible
//! Oracle: When The One Ring enters, if you cast it, you gain protection from everything until your next turn.
//! Oracle: At the beginning of your upkeep, you lose 1 life for each burden counter on The One Ring.
//! Oracle: {T}: Put a burden counter on The One Ring, then draw a card for each burden counter on The One Ring.
//! Set: LTR #246 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: d5806e68-1054-458e-866d-1f2470f682b2 | Oracle ID: 3aa83ed2-f48b-4ce6-a614-2c54ddf50538

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THE_ONE_RING,
    oracle_id = "3aa83ed2-f48b-4ce6-a614-2c54ddf50538",
    scryfall_id = "d5806e68-1054-458e-866d-1f2470f682b2",
    faces = &[face!(
        name = "The One Ring",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    keywords = KeywordSet::INDESTRUCTIBLE,
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::continuous(
                &Filter::Any,
                Modifier::PlayerProtectionFromEverything {
                    who: PlayerRel::You
                },
                Duration::UntilYourNextTurn
            )],
            condition = Some(Condition::SourceWasCastByYou)
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::LoseLife {
                amount: Amount::CountersOnSource(counters::BURDEN),
                target: PlayerRel::You
            }]
        ),
        activated!(
            cost!(TapSelf),
            &[
                Effect::AddCounter {
                    kind: counters::BURDEN,
                    amount: Amount::Fixed(1)
                },
                Effect::DrawCards {
                    amount: Amount::CountersOnSource(counters::BURDEN)
                }
            ]
        ),
    ],
);
