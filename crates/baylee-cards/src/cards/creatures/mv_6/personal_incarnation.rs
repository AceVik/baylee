//! Personal Incarnation — {3}{W}{W}{W} — Creature — Avatar Incarnation
//! Oracle: {0}: The next 1 damage that would be dealt to this creature this turn is dealt to its owner instead. Only this creature's owner may activate this ability.
//! Oracle: When this creature dies, its owner loses half their life, rounded up.
//! Set: ME4 #22 — Masters Edition IV | Scryfall ID: 456b44ec-c299-419a-82b6-99d8609a0c04 | Oracle ID: 6e49a5b8-6bc4-4c7b-82c1-957f1fb0ca5f
// PARTIAL — rules implemented; independent acceptance and live validation pending.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PERSONAL_INCARNATION,
    oracle_id = "6e49a5b8-6bc4-4c7b-82c1-957f1fb0ca5f",
    scryfall_id = "456b44ec-c299-419a-82b6-99d8609a0c04",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial("independent acceptance and live validation pending"),
    faces = &[face!(
        name = "Personal Incarnation",
        mana_cost = mana!("{3}{W}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::AVATAR, subtypes::creature::INCARNATION],
        power = Some(6),
        toughness = Some(6),
    ),],
    abilities = &[
        activated!(
            Cost::FREE,
            &[Effect::RedirectNextDamage {
                target: TargetSpec::ThisObject,
                amount: Amount::Fixed(1),
                to: PlayerRel::OwnerOfSource,
            }],
            zone = ActivationZone::BattlefieldOwner
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::LoseHalfLife {
                player: PlayerRel::OwnerOfSource
            }]
        ),
    ],
);
