//! Oath of Druids — {1}{G} — Enchantment
//! Oracle: At the beginning of each player's upkeep, that player chooses target player who controls more creatures than they do and is their opponent. The first player may reveal cards from the top of their library until they reveal a creature card. If the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard.
//! Set: C16 #159 — Commander 2016 | Scryfall ID: 9dad6b50-c415-4c55-8eac-bbc9d656c2fc | Oracle ID: cf0dcb27-f02e-4065-81dc-75fbbb09ea3f

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OATH_OF_DRUIDS,
    oracle_id = "cf0dcb27-f02e-4065-81dc-75fbbb09ea3f",
    scryfall_id = "9dad6b50-c415-4c55-8eac-bbc9d656c2fc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Triggering for each player to choose a target opponent with more creatures and put non-creature revealed cards into their graveyard is not supported; at the beginning of your upkeep, you may target an opponent and reveal cards from your library until you reveal a creature card, putting it onto the battlefield and the rest on the bottom in a random order."
    ),
    faces = &[face!(
        name = "Oath of Druids",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[Effect::RevealUntil {
                filter: &Filter::CREATURE,
                found: SearchDest::Battlefield,
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
