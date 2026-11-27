//! Spoils of Evil — {2}{B} — Instant
//! Oracle: For each artifact or creature card in target opponent's graveyard, add {C} and you gain 1 life.
//! Set: ICE #163 — Ice Age | Scryfall ID: fd368eb6-72f0-42d4-afa5-3daa7de949ff | Oracle ID: a40c52ee-4b45-4b14-8fbc-6529b42bfc9d
// IMPLEMENTED — targets an opponent and counts artifact and creature cards in their graveyard for {C} and life.

use baylee_cards_dsl::prelude::*;

static ARTIFACT_OR_CREATURE: Filter = Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE]);

static TARGET_GRAVEYARD_COUNT: Amount = Amount::CountOf {
    filter: &ARTIFACT_OR_CREATURE,
    zone: ZoneSel::GraveyardOf(PlayerRel::Chosen),
};

card!(
    index = index::SPOILS_OF_EVIL,
    oracle_id = "a40c52ee-4b45-4b14-8fbc-6529b42bfc9d",
    scryfall_id = "fd368eb6-72f0-42d4-afa5-3daa7de949ff",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spoils of Evil",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::mana_dynamic(ManaColor::Colorless, TARGET_GRAVEYARD_COUNT),
            Effect::GainLife {
                amount: TARGET_GRAVEYARD_COUNT,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
