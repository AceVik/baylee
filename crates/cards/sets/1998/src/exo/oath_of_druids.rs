//! Oath of Druids — {1}{G} — Enchantment
//! Oracle: At the beginning of each player's upkeep, that player chooses target player who controls more creatures than they do and is their opponent. The first player may reveal cards from the top of their library until they reveal a creature card. If the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard.
//! Set: C16 #159 — Commander 2016 | Scryfall ID: 9dad6b50-c415-4c55-8eac-bbc9d656c2fc | Oracle ID: cf0dcb27-f02e-4065-81dc-75fbbb09ea3f
// IMPLEMENTED — the active player targets an opposing player with more
// creatures, then may reveal to a creature and put it onto the battlefield.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OATH_OF_DRUIDS,
    oracle_id = "cf0dcb27-f02e-4065-81dc-75fbbb09ea3f",
    scryfall_id = "9dad6b50-c415-4c55-8eac-bbc9d656c2fc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Oath of Druids",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::MayDoFor {
            player: PlayerRel::ActivePlayer,
            effects: &[Effect::RevealUntilMatching {
                player: PlayerRel::ActivePlayer,
                filter: &Filter::CREATURE,
                matched: SearchDest::Battlefield,
                otherwise: SearchDest::Graveyard,
            }],
        }],
        targets = Some(
            TargetReq::one(TargetSpec::OpponentWithMoreThanActivePlayer(
                &Filter::CREATURE
            ),)
            .chosen_by_event_player()
        ),
    )],
);
