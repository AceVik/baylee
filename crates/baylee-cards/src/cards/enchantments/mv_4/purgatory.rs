//! Purgatory — {2}{W}{B} — Enchantment
//! Oracle: Whenever a nontoken creature is put into your graveyard from the battlefield, exile that card.
//! Oracle: At the beginning of your upkeep, you may pay {4} and 2 life. If you do, return a card exiled with this enchantment to the battlefield.
//! Set: MIR #275 — Mirage | Scryfall ID: 9ed5e58b-46f3-437c-89ac-24235a65bd1f | Oracle ID: 2fb8e100-58a1-4dc6-bc38-ad7f115e3e46
// PARTIAL: the upkeep payment is {4} paid and then 2 life lost as an effect, rather than {4} and 2 life as one combined cost (at 1 life the printed payment is impossible, here it would be paid and the player would lose); the exiled creatures are your own nontoken creatures put into your graveyard from the battlefield

use baylee_cards_dsl::prelude::*;

/// "A nontoken creature … put into your graveyard": a card goes to its owner's graveyard.
const NONTOKEN_CREATURE_YOU_OWN: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::OwnedByYou,
    Filter::Not(&Filter::IsToken),
]);

card!(
    index = index::PURGATORY,
    oracle_id = "2fb8e100-58a1-4dc6-bc38-ad7f115e3e46",
    scryfall_id = "9ed5e58b-46f3-437c-89ac-24235a65bd1f",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    coverage = Coverage::Partial(
        "the upkeep payment is {4} paid and then 2 life lost as an effect, rather than {4} and 2 life as one combined cost (at 1 life the printed payment is impossible, here it would be paid and the player would lose); the exiled creatures are your own nontoken creatures put into your graveyard from the battlefield"
    ),
    faces = &[face!(
        name = "Purgatory",
        mana_cost = mana!("{2}{W}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::Dies(&NONTOKEN_CREATURE_YOU_OWN),
            &[Effect::exile_linked(TargetSpec::Object(&Filter::Any))],
            targets = Some(TargetReq::one(TargetSpec::EventObject)),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayThen {
                player: PlayerRel::You,
                mana: Amount::Fixed(4),
                effects: &[
                    Effect::LoseLife {
                        amount: Amount::Fixed(2),
                        target: PlayerRel::You,
                    },
                    Effect::ReturnChosenLinkedToBattlefield {
                        filter: &Filter::Any,
                        other_to_hand: false,
                    },
                ],
            }],
        ),
    ],
);
