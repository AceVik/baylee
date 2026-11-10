//! Defiant Vanguard — {2}{W} — Creature — Human Rebel
//! Oracle: When this creature blocks, at end of combat, destroy it and all creatures it blocked this turn.
//! Oracle: {5}, {T}: Search your library for a Rebel permanent card with mana value 4 or less, put it onto the battlefield, then shuffle.
//! Set: TSB #5 — Time Spiral Timeshifted | Scryfall ID: a37acadc-0e58-4f44-93f4-dd465b9ee06f | Oracle ID: 680766e2-637b-40bc-a15f-0b4dc55bd619
// IMPLEMENTED — end-of-combat mutual destruction on block + Rebel recruitment.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLOCKED_BY_IT: Filter = Filter::And(&[Filter::CREATURE, Filter::BlockedByReferenceThisTurn]);
static REBEL_MV_AT_MOST_4: Filter = Filter::And(&[
    Filter::HasType(
        TypeSet::ARTIFACT
            .union(TypeSet::BATTLE)
            .union(TypeSet::CREATURE)
            .union(TypeSet::ENCHANTMENT)
            .union(TypeSet::LAND)
            .union(TypeSet::PLANESWALKER),
    ),
    Filter::HasSubtype(subtypes::creature::REBEL),
    Filter::CmcAtMost(4),
]);

card!(
    index = index::DEFIANT_VANGUARD,
    oracle_id = "680766e2-637b-40bc-a15f-0b4dc55bd619",
    scryfall_id = "a37acadc-0e58-4f44-93f4-dd465b9ee06f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Defiant Vanguard",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::REBEL],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        triggered!(
            Trigger::Blocks(&Filter::This),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::ThisObject,
                effects: &[
                    Effect::destroy(TargetSpec::ThisObject),
                    Effect::destroy_all(&BLOCKED_BY_IT),
                ],
            }],
        ),
        activated!(
            cost!("{5}", TapSelf),
            &[Effect::SearchLibrary {
                filter: &REBEL_MV_AT_MOST_4,
                finds: &[Find::BATTLEFIELD],
                optional: false,
            }],
        ),
    ],
);
