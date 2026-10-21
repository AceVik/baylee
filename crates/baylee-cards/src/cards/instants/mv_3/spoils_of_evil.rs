//! Spoils of Evil — {2}{B} — Instant
//! Oracle: For each artifact or creature card in target opponent's graveyard, add {C} and you gain 1 life.
//! Set: ICE #163 — Ice Age | Scryfall ID: fd368eb6-72f0-42d4-afa5-3daa7de949ff | Oracle ID: a40c52ee-4b45-4b14-8fbc-6529b42bfc9d

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPOILS_OF_EVIL,
    oracle_id = "a40c52ee-4b45-4b14-8fbc-6529b42bfc9d",
    scryfall_id = "fd368eb6-72f0-42d4-afa5-3daa7de949ff",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "targets an opponent and adds {C} and 1 life per artifact or creature card; at a table of more than two players, it counts across all opponents' graveyards instead of only the targeted opponent's graveyard",
    ),
    faces = &[face!(
        name = "Spoils of Evil",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        // NOT SUPPORTED: ZoneSel for a targeted player's graveyard at a multiplayer table (ZoneSel::GraveyardOpponents counts across all opponents' graveyards).
        // CR 106.1 / CR 119.1: For each artifact or creature card in target opponent's graveyard, add {C} and you gain 1 life.
        spell!(
            &[
                Effect::mana_dynamic(
                    ManaColor::Colorless,
                    Amount::CountOf {
                        filter: &Filter::ARTIFACT_OR_CREATURE,
                        zone: ZoneSel::GraveyardOpponents,
                    },
                ),
                Effect::GainLife {
                    amount: Amount::CountOf {
                        filter: &Filter::ARTIFACT_OR_CREATURE,
                        zone: ZoneSel::GraveyardOpponents,
                    },
                },
            ],
            targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
        ),
    ],
);
