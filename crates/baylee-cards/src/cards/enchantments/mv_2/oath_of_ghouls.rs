//! Oath of Ghouls — {1}{B} — Enchantment
//! Oracle: At the beginning of each player's upkeep, that player chooses target player whose graveyard has fewer creature cards in it than their graveyard does and is their opponent. The first player may return a creature card from their graveyard to their hand.
//! Set: EXO #69 — Exodus | Scryfall ID: 1102f35a-ae62-479d-b61c-31a82978aedd | Oracle ID: b9490b52-6c3f-437b-b2c2-9e81c4b43b61

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OATH_OF_GHOULS,
    oracle_id = "b9490b52-6c3f-437b-b2c2-9e81c4b43b61",
    scryfall_id = "1102f35a-ae62-479d-b61c-31a82978aedd",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Triggering for each player and restricting the target opponent to one with fewer creature cards in their graveyard is not supported; at the beginning of your upkeep, you may return target creature card from your graveyard to your hand."
    ),
    faces = &[face!(
        name = "Oath of Ghouls",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        ))),
    )],
);
