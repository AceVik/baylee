//! Enduring Vitality — {1}{G}{G} — Enchantment Creature — Elk Glimmer
//! Oracle: Vigilance
//! Oracle: Creatures you control have "{T}: Add one mana of any color."
//! Oracle: When Enduring Vitality dies, if it was a creature, return it to the battlefield under its owner's control. It's an enchantment. (It's not a creature.)
//! Set: DSK #176 — Duskmourn: House of Horror | Scryfall ID: 9d76a30c-0431-4334-892a-9822dda9671a | Oracle ID: 3577c47e-76d3-4659-b922-31c4b74be3a0
// PARTIAL — vigilance on the face and the granted "{T}: Add one mana of any
// color." on your creatures, both worth a game; the die trigger is not
// expressible and is left off with a NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENDURING_VITALITY,
    oracle_id = "3577c47e-76d3-4659-b922-31c4b74be3a0",
    scryfall_id = "9d76a30c-0431-4334-892a-9822dda9671a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::VIGILANCE,
    coverage = Coverage::Partial(
        "the die trigger: no Effect returns the source card from its graveyard \
         to the battlefield and changes what the permanent becomes there (the \
         enduring rider — an enchantment that is not a creature), and no \
         Condition states the printed intervening \"if it was a creature\""
    ),
    faces = &[face!(
        name = "Enduring Vitality",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::ENCHANTMENT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::ELK, subtypes::creature::GLIMMER],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[
        static_ability!(
            Filter::YOUR_CREATURE,
            Modifier::GrantActivated {
                cost: Cost::TAP,
                effects: &[Effect::mana_of_any_color()],
                mana_ability: true,
            }
        ),
        // NOT SUPPORTED: "When Enduring Vitality dies, if it was a creature,
        // return it to the battlefield under its owner's control. It's an
        // enchantment. (It's not a creature.)" — `Trigger::Dies` names the
        // event, but nothing carries the rest of the sentence: no Effect drops
        // the card back onto the battlefield from a graveyard *and* rewrites
        // what the arriving permanent is (the enduring rider — an enchantment
        // and not a creature), and no `Condition` answers the intervening "if
        // it was a creature", which is about the object as it was and not
        // about the source object the ability is a separate thing from.
    ],
);
