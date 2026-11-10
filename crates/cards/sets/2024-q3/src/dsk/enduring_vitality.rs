//! Enduring Vitality — {1}{G}{G} — Enchantment Creature — Elk Glimmer
//! Oracle: Vigilance
//! Oracle: Creatures you control have "{T}: Add one mana of any color."
//! Oracle: When Enduring Vitality dies, if it was a creature, return it to the battlefield under its owner's control. It's an enchantment. (It's not a creature.)
//! Set: DSK #176 — Duskmourn: House of Horror | Scryfall ID: 9d76a30c-0431-4334-892a-9822dda9671a | Oracle ID: 3577c47e-76d3-4659-b922-31c4b74be3a0
// IMPLEMENTED — vigilance, the granted "{T}: Add one mana of any color." on
// your creatures, and the enduring return: it dies as a creature, comes back
// under its owner's control, and is an enchantment and no creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENDURING_VITALITY,
    oracle_id = "3577c47e-76d3-4659-b922-31c4b74be3a0",
    scryfall_id = "9d76a30c-0431-4334-892a-9822dda9671a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::VIGILANCE,
    coverage = Coverage::Implemented,
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
        // "When Enduring Vitality dies, if it was a creature, return it to
        // the battlefield under its owner's control. It's an enchantment.
        // (It's not a creature.)" The dying object is read as it last was
        // (CR 603.10a), so a filter that asks for a creature *is* the
        // intervening "if it was a creature": its answer cannot change
        // between the trigger and its resolution. `This` in the continuous
        // effect is the source, which after the return is the new permanent;
        // it is registered only if the card did arrive.
        triggered!(
            Trigger::Dies(&Filter::And(&[Filter::This, Filter::CREATURE])),
            &[
                Effect::GraveyardToBattlefield {
                    target: TargetSpec::EventObject,
                    owner_control: true,
                    counters: None,
                },
                Effect::IfCondition {
                    condition: Condition::SourceMatches(&Filter::InZone(ZoneRef::Battlefield)),
                    then: &[Effect::continuous(
                        &Filter::This,
                        Modifier::RemoveType(TypeSet::CREATURE),
                        Duration::Indefinitely,
                    )],
                    otherwise: &[],
                },
            ]
        ),
    ],
);
