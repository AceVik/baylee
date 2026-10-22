//! Purgatory — {2}{W}{B} — Enchantment
//! Oracle: Whenever a nontoken creature is put into your graveyard from the battlefield, exile that card.
//! Oracle: At the beginning of your upkeep, you may pay {4} and 2 life. If you do, return a card exiled with this enchantment to the battlefield.
//! Set: MIR #275 — Mirage | Scryfall ID: 9ed5e58b-46f3-437c-89ac-24235a65bd1f | Oracle ID: 2fb8e100-58a1-4dc6-bc38-ad7f115e3e46
// PARTIAL: Trigger::Dies checks all dying nontoken creatures rather than only those put into your graveyard, and the upkeep trigger pays {4} and then loses 2 life rather than paying {4} and 2 life as a single combined cost; dying nontoken creatures are exiled linked to this enchantment and paying {4} at your upkeep loses 2 life and returns a chosen linked card to the battlefield

use baylee_cards_dsl::prelude::*;

const NONTOKEN_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::IsToken)]);

card!(
    index = index::PURGATORY,
    oracle_id = "2fb8e100-58a1-4dc6-bc38-ad7f115e3e46",
    scryfall_id = "9ed5e58b-46f3-437c-89ac-24235a65bd1f",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    coverage = Coverage::Partial(
        "Trigger::Dies checks all dying nontoken creatures rather than only those put into your graveyard, and the upkeep trigger pays {4} and then loses 2 life rather than paying {4} and 2 life as a single combined cost; dying nontoken creatures are exiled linked to this enchantment and paying {4} at your upkeep loses 2 life and returns a chosen linked card to the battlefield"
    ),
    faces = &[face!(
        name = "Purgatory",
        mana_cost = mana!("{2}{W}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::Dies(&NONTOKEN_CREATURE),
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
