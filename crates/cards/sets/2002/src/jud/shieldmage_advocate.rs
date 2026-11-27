//! Shieldmage Advocate — {2}{W} — Creature — Human Cleric
//! Oracle: {T}: Return target card from an opponent's graveyard to their hand. Prevent all damage that would be dealt to any target this turn by a source of your choice.
//! Set: JUD #22 — Judgment | Scryfall ID: 2ea66a41-cb2e-49d6-81fe-3f69b0dfd40e | Oracle ID: 57188b3d-567c-4dff-8b93-7cc7a47894be
// IMPLEMENTED — returns opponent graveyard card to hand and prevents all damage from chosen source to second target this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHIELDMAGE_ADVOCATE,
    oracle_id = "57188b3d-567c-4dff-8b93-7cc7a47894be",
    scryfall_id = "2ea66a41-cb2e-49d6-81fe-3f69b0dfd40e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shieldmage Advocate",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::Opponent),
            },
            Effect::WithSecondTarget(&[Effect::PreventFromChosenSource {
                target: Some(TargetSpec::AnyTarget),
                sources: &Filter::Any,
                next_only: false,
                combat_only: false,
                all_but: 0,
                gain_life: false,
            }]),
        ],
        target = Some(TargetSpec::CardInGraveyard(
            &Filter::Any,
            PlayerRel::Opponent
        )),
        second_targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
