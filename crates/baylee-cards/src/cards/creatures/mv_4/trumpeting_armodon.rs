//! Trumpeting Armodon — {3}{G} — Creature — Elephant
//! Oracle: {1}{G}: Target creature blocks this creature this turn if able.
//! Set: TMP #262 — Tempest | Scryfall ID: 38f94fd1-6f85-41ad-9674-f05cc893324f | Oracle ID: 7fc3c2a5-2754-4237-8a37-cb3166c20c3c
// PARTIAL: activates {1}{G} to require target creature to block each attacking creature this turn if able; requiring target creature to block specifically this creature this turn if able is not on Modifier.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TRUMPETING_ARMODON,
    oracle_id = "7fc3c2a5-2754-4237-8a37-cb3166c20c3c",
    scryfall_id = "38f94fd1-6f85-41ad-9674-f05cc893324f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "activates {1}{G} to require target creature to block each attacking creature this turn if able; requiring target creature to block specifically this creature this turn if able is not on Modifier",
    ),
    faces = &[face!(
        name = "Trumpeting Armodon",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEPHANT],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{1}{G}"),
        &[Effect::ModifyTargets {
            modifier: Modifier::BlocksEachAttackerIfAble,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
