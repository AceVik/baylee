//! Helm of Obedience — {4} — Artifact
//! Oracle: {X}, {T}: Target opponent mills a card, then repeats this process until a creature card or X cards have been put into their graveyard this way, whichever comes first. If one or more creature cards were put into that graveyard this way, sacrifice this artifact and put one of them onto the battlefield under your control. X can't be 0.
//! Set: ME2 #210 — Masters Edition II | Scryfall ID: ec837c3e-a66f-43c0-adb4-4eefe7888cac | Oracle ID: 16cadebf-c484-41f8-9e38-5c2c528f5b54
// NOT SUPPORTED: iteratively milling until a creature card or X cards are put into the graveyard, sacrificing this artifact to put one of those creature cards onto the battlefield under your control, and restricting X from being 0 are not supported; {X}, {T} causes target opponent to mill X cards.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HELM_OF_OBEDIENCE,
    oracle_id = "16cadebf-c484-41f8-9e38-5c2c528f5b54",
    scryfall_id = "ec837c3e-a66f-43c0-adb4-4eefe7888cac",
    coverage = Coverage::Partial(
        "iteratively milling until a creature card or X cards are put into the graveyard, sacrificing this artifact to put one of those creature cards onto the battlefield under your control, and restricting X from being 0 are not supported; {X}, {T} causes target opponent to mill X cards",
    ),
    faces = &[face!(
        name = "Helm of Obedience",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{X}", TapSelf),
        &[Effect::Mill {
            amount: Amount::X,
            target: PlayerRel::Chosen,
        }],
        target = Some(TargetSpec::AnyOpponent),
    )],
);
