//! Sevinne's Reclamation — {2}{W} — Sorcery
//! Oracle: Return target permanent card with mana value 3 or less from your graveyard to the battlefield. If this spell was cast from a graveyard, you may copy this spell and may choose a new target for the copy.
//! Oracle: Flashback {4}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: SOC #170 — Secrets of Strixhaven Commander | Scryfall ID: 8deab1ef-4219-4767-a3c4-b61250d0ebe0 | Oracle ID: 1b9f9f5b-8712-4f00-90cb-1b7b9970eccc
// IMPLEMENTED — return, flashback, and optional graveyard-cast copy; canonical
// scenarios, independent review, and mutation checks passed.

use baylee_cards_dsl::prelude::*;

/// "Permanent card with mana value 3 or less" — the same three clauses Sun
/// Titan writes for the same printed sentence, so the two cards are provably
/// reading one filter and not two: in this pool a permanent card is a card
/// that is neither an instant nor a sorcery.
static SMALL_PERMANENT: Filter = Filter::And(&[
    Filter::CmcAtMost(3),
    Filter::LacksType(TypeSet::INSTANT),
    Filter::LacksType(TypeSet::SORCERY),
]);

card!(
    index = index::SEVINNE_S_RECLAMATION,
    oracle_id = "1b9f9f5b-8712-4f00-90cb-1b7b9970eccc",
    scryfall_id = "8deab1ef-4219-4767-a3c4-b61250d0ebe0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sevinne's Reclamation",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::SORCERY,
        flashback = Some(mana!("{4}{W}")),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            Effect::reanimate(TargetSpec::CardInGraveyard(
                &SMALL_PERMANENT,
                PlayerRel::You,
            )),
            Effect::IfCastFromGraveyard {
                effects: &[Effect::MayDo {
                    effects: &[Effect::CopyResolvingSpell {
                        controller: PlayerRel::You,
                    }],
                }],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &SMALL_PERMANENT,
            PlayerRel::You,
        )))
    )],
);
