//! Gustha's Scepter — {0} — Artifact
//! Oracle: {T}: Exile a card from your hand face down. You may look at it for as long as it remains exiled.
//! Oracle: {T}: Return a card you own exiled with this artifact to your hand.
//! Oracle: When you lose control of this artifact, put all cards exiled with this artifact into their owner's graveyard.
//! Set: ME2 #209 — Masters Edition II | Scryfall ID: 7d4f3248-2b3f-42b3-9a9f-77309bb46c34 | Oracle ID: 120e9013-b07b-49d9-8b86-f6c26315581d
// NOT SUPPORTED: exiling a card from your hand face down linked to this artifact, returning a single chosen linked exiled card to your hand, and triggering on losing control of this artifact are not supported; first ability exiles the top card of your library face down linked to this artifact, second ability returns all linked exiled cards to their owners' hands, and leaves-the-battlefield trigger puts linked exiled cards into their owners' graveyards.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GUSTHA_S_SCEPTER,
    oracle_id = "120e9013-b07b-49d9-8b86-f6c26315581d",
    scryfall_id = "7d4f3248-2b3f-42b3-9a9f-77309bb46c34",
    coverage = Coverage::Partial(
        "exiling a card from your hand face down linked to this artifact, returning a single chosen linked exiled card to your hand, and triggering on losing control of this artifact are not supported; first ability exiles the top card of your library face down linked to this artifact, second ability returns all linked exiled cards to their owners' hands, and leaves-the-battlefield trigger puts linked exiled cards into their owners' graveyards",
    ),
    faces = &[face!(
        name = "Gustha's Scepter",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        activated!(Cost::TAP, &[Effect::ExileTopFaceDownLinked]),
        activated!(
            Cost::TAP,
            &[Effect::ReturnLinkedExile { to: ZoneRef::Hand }]
        ),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::ReturnLinkedExile {
                to: ZoneRef::Graveyard,
            }],
        ),
    ],
);
