//! Pygmy Troll — {1}{G} — Creature — Troll
//! Oracle: Whenever this creature becomes blocked by a creature, this creature gets +1/+1 until end of turn.
//! Oracle: {G}: Regenerate this creature.
//! Set: EXO #118 — Exodus | Scryfall ID: 7be9714d-125f-4700-879d-b920fe9f1b68 | Oracle ID: 6f18f41e-df8d-4b59-87cc-ea167823fbda
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PYGMY_TROLL,
    oracle_id = "6f18f41e-df8d-4b59-87cc-ea167823fbda",
    scryfall_id = "7be9714d-125f-4700-879d-b920fe9f1b68",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pygmy Troll",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TROLL],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::BecomesBlockedBy(&Filter::CREATURE),
            &[Effect::ModifySource {
                modifier: Modifier::ModifyPT(1, 1),
                duration: Duration::UntilEndOfTurn,
            }]
        ),
        activated!(cost!("{G}"), &[Effect::regenerate(TargetSpec::ThisObject)]),
    ],
);
