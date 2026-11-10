//! Enduring Renewal — {2}{W}{W} — Enchantment
//! Oracle: Play with your hand revealed.
//! Oracle: If you would draw a card, reveal the top card of your library instead. If it's a creature card, put it into your graveyard. Otherwise, draw a card.
//! Oracle: Whenever a creature is put into your graveyard from the battlefield, return it to your hand.
//! Set: TSB #7 — Time Spiral Timeshifted | Scryfall ID: 57a966e3-c9ba-4105-a36b-54ca70ba9b77 | Oracle ID: 47a080c4-ff04-4f52-aca0-2b8e4f4d931e

use baylee_cards_dsl::prelude::*;

const YOUR_OWNED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::OwnedByYou]);

card!(
    index = index::ENDURING_RENEWAL,
    oracle_id = "47a080c4-ff04-4f52-aca0-2b8e4f4d931e",
    scryfall_id = "57a966e3-c9ba-4105-a36b-54ca70ba9b77",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Enduring Renewal",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(Filter::Any, Modifier::RevealYourHand),
        AbilityDef::Replacement(ReplacementRule::RevealDrawPutMatchingInGraveyard(
            &Filter::CREATURE,
        )),
        triggered!(
            Trigger::Dies(&YOUR_OWNED_CREATURE),
            &[Effect::ReturnToHand {
                target: TargetSpec::EventObject,
            }],
        ),
    ],
);
