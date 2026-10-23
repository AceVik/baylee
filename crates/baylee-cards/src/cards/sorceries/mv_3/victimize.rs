//! Victimize — {2}{B} — Sorcery
//! Oracle: Choose two target creature cards in your graveyard. Sacrifice a creature. If you do, return the chosen cards to the battlefield tapped.
//! Set: USG #166 — Urza's Saga | Scryfall ID: caafe7da-0167-4c53-bbad-172f900d137b | Oracle ID: 240e85d3-e495-4877-8609-4b4056c402f7
// IMPLEMENTED — sacrifice on resolution; only on success return both targets tapped.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VICTIMIZE,
    oracle_id = "240e85d3-e495-4877-8609-4b4056c402f7",
    scryfall_id = "caafe7da-0167-4c53-bbad-172f900d137b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Victimize",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::SacrificeOneThen {
            filter: &Filter::YOUR_CREATURE,
            if_it_was: &Filter::CREATURE,
            then: &[Effect::ReturnToBattlefieldTapped {
                target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            }],
        }],
        targets = Some(TargetReq::exactly(
            TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            2,
        )),
    )],
);
