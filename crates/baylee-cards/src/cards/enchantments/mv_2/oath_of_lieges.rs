//! Oath of Lieges — {1}{W} — Enchantment
//! Oracle: At the beginning of each player's upkeep, that player chooses target player who controls more lands than they do and is their opponent. The first player may search their library for a basic land card, put that card onto the battlefield, then shuffle.
//! Set: EXO #11 — Exodus | Scryfall ID: 470a2092-eeda-4557-8cee-ac401b61a225 | Oracle ID: 87038354-6762-4069-be34-7d40ae308a8d
// PARTIAL: Triggers on your upkeep targeting any opponent to let you search your library for a basic land card and put it onto the battlefield; triggering for each player and restricting target opponent to one who controls more lands are not supported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OATH_OF_LIEGES,
    oracle_id = "87038354-6762-4069-be34-7d40ae308a8d",
    scryfall_id = "470a2092-eeda-4557-8cee-ac401b61a225",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Triggers on your upkeep targeting any opponent to let you search your library for a basic land card and put it onto the battlefield; triggering for each player and restricting target opponent to one who controls more lands are not supported.",
    ),
    faces = &[face!(
        name = "Oath of Lieges",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[Effect::SearchLibrary {
                filter: &Filter::BASIC_LAND,
                finds: &[Find::BATTLEFIELD],
                optional: true,
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
    ),],
);
