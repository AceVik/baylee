//! Krovikan Horror — {3}{B} — Creature — Horror Spirit
//! Oracle: At the beginning of the end step, if this card is in your graveyard with a creature card directly above it, you may return this card to your hand.
//! Oracle: {1}, Sacrifice a creature: This creature deals 1 damage to any target.
//! Set: ME2 #101 — Masters Edition II | Scryfall ID: 8321977d-69bf-4e10-a776-8d714b0b9d46 | Oracle ID: a4c15223-784f-43a7-8626-30f96ccfdcaf
// NOT SUPPORTED: checking whether a creature card is directly above this card in your graveyard is approximated by checking whether at least one creature card is above it in your graveyard.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KROVIKAN_HORROR,
    oracle_id = "a4c15223-784f-43a7-8626-30f96ccfdcaf",
    scryfall_id = "8321977d-69bf-4e10-a776-8d714b0b9d46",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "at the beginning of the end step, if it is in your graveyard with at least one creature card above it, you may return it to your hand, and {1}, sacrifice a creature deals 1 damage to any target; requiring a creature card to be directly above it rather than anywhere above it in your graveyard is not supported",
    ),
    faces = &[face!(
        name = "Krovikan Horror",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORROR, subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::MayDo {
                effects: &[Effect::ReturnToHand {
                    target: TargetSpec::ThisObject,
                }],
            }],
            zone = TriggerZone::Graveyard,
            condition = Some(Condition::GraveyardCardsAbove(&Filter::CREATURE, 1)),
        ),
        activated!(
            cost!("{1}", Sacrifice(&Filter::YOUR_CREATURE)),
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
            }],
            target = Some(TargetSpec::AnyTarget),
        ),
    ],
);
