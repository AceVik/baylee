//! Knowledge Vault — {4} — Artifact
//! Oracle: {2}, {T}: Exile the top card of your library face down.
//! Oracle: {0}: Sacrifice this artifact. If you do, discard your hand, then put all cards exiled with this artifact into their owner's hand.
//! Oracle: When this artifact leaves the battlefield, put all cards exiled with it into their owner's graveyard.
//! Set: ME3 #198 — Masters Edition III | Scryfall ID: c416fca6-e201-4dbc-b9c7-248d70967746 | Oracle ID: 77d3cc31-837a-408a-b7f8-5aa9de78e1b7
// IMPLEMENTED — privately linked exile, resolution-time sacrifice and ordered returns.
// Behavior: baylee-engine/src/engine/card_tests/hidden_exile.rs.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KNOWLEDGE_VAULT,
    oracle_id = "77d3cc31-837a-408a-b7f8-5aa9de78e1b7",
    scryfall_id = "c416fca6-e201-4dbc-b9c7-248d70967746",
    faces = &[face!(
        name = "Knowledge Vault",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        activated!(cost!("{2}", TapSelf), &[Effect::ExileTopFaceDownLinked]),
        activated!(
            cost!("{0}"),
            &[Effect::SacrificeSelfThen {
                then: &[
                    Effect::DiscardHand {
                        who: PlayerRel::You
                    },
                    Effect::ReturnLinkedExile { to: ZoneRef::Hand },
                ],
            }]
        ),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::ReturnLinkedExile {
                to: ZoneRef::Graveyard
            },]
        ),
    ],
);
