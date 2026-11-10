//! Legion's Landing // Adanto, the First Fort — {W} — Legendary Enchantment // Legendary Land
//! Oracle: When Legion's Landing enters, create a 1/1 white Vampire creature token with lifelink.
//! Oracle: When you attack with three or more creatures, transform Legion's Landing.
//! Oracle: (Transforms from Legion's Landing.)
//! Oracle: {T}: Add {W}.
//! Oracle: {2}{W}, {T}: Create a 1/1 white Vampire creature token with lifelink.
//! Set: XLN #22 — Ixalan | Scryfall ID: 05e2a5e6-3aaa-4096-bdd0-fcc1afe5a36c | Oracle ID: f7d8b91b-6541-4d3e-af51-7e000eac69c1
//! Face: Legion's Landing — {W} — Legendary Enchantment
//! Face: Adanto, the First Fort —  — Legendary Land

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
static BACK_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
    activated!(
        cost!("{2}{W}", TapSelf),
        &[Effect::CreateToken {
            token: &generated_tokens::VAMPIRE_1_1_WHITE_LIFELINK
        }]
    ),
];

card!(
    index = index::LEGION_S_LANDING,
    oracle_id = "f7d8b91b-6541-4d3e-af51-7e000eac69c1",
    scryfall_id = "05e2a5e6-3aaa-4096-bdd0-fcc1afe5a36c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[
        face!(
            name = "Legion's Landing",
            mana_cost = mana!("{W}"),
            types = TypeSet::ENCHANTMENT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Adanto, the First Fort",
            castable_from_hand = false,
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            abilities = BACK_ABILITIES,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::CreateToken {
                token: &generated_tokens::VAMPIRE_1_1_WHITE_LIFELINK
            }]
        ),
        triggered!(Trigger::AttackWithAtLeast(3), &[Effect::TransformSource]),
    ],
);
