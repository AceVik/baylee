//! Victimize — {2}{B} — Sorcery
//! Oracle: Choose two target creature cards in your graveyard. Sacrifice a creature. If you do, return the chosen cards to the battlefield tapped.
//! Set: USG #166 — Urza's Saga | Scryfall ID: caafe7da-0167-4c53-bbad-172f900d137b | Oracle ID: 240e85d3-e495-4877-8609-4b4056c402f7
// PARTIAL: returning the chosen cards to the battlefield tapped is not in GraveyardToBattlefield; chooses two target creature cards in your graveyard, sacrifices a creature, and if you do, returns the chosen cards to the battlefield untapped.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VICTIMIZE,
    oracle_id = "240e85d3-e495-4877-8609-4b4056c402f7",
    scryfall_id = "caafe7da-0167-4c53-bbad-172f900d137b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "returning the chosen cards to the battlefield tapped is not in GraveyardToBattlefield; chooses two target creature cards in your graveyard, sacrifices a creature, and if you do, returns the chosen cards to the battlefield untapped",
    ),
    faces = &[face!(
        name = "Victimize",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::SacrificeFilter {
                who: PlayerRel::You,
                filter: &Filter::YOUR_CREATURE,
            },
            Effect::IfCondition {
                condition: Condition::AmountAtLeast(
                    Amount::ThisWay(ThisWay::Died(&Filter::Any)),
                    1,
                ),
                then: &[Effect::reanimate(TargetSpec::CardInGraveyard(
                    &Filter::CREATURE,
                    PlayerRel::You,
                ))],
                otherwise: &[],
            },
        ],
        targets = Some(TargetReq::exactly(
            TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            2,
        )),
    )],
);
