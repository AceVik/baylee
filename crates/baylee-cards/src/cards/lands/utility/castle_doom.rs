//! Castle Doom — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast an artifact spell.
//! Oracle: {3}, {T}, Sacrifice an artifact: Create a 3/3 colorless Robot Villain artifact creature token named Doombot. Activate only as a sorcery.
//! Set: MSH #263 — Marvel Super Heroes | Scryfall ID: 6b39d7a6-ca2d-4376-a18e-efd0138e83bc | Oracle ID: 9bd013df-ad75-4099-940b-1765c58faf26
// PARTIAL — both mana abilities are written, the {C} one plain and the
// any-colour one restricted to artifact spells; the Doombot ability is not,
// because the token it makes has no `TokenDef`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CASTLE_DOOM,
    oracle_id = "9bd013df-ad75-4099-940b-1765c58faf26",
    scryfall_id = "6b39d7a6-ca2d-4376-a18e-efd0138e83bc",
    faces = &[face!(name = "Castle Doom", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "no Doombot token in crate::tokens, so the {3}, {T}, Sacrifice an \
         artifact ability is not written"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana_of_any_color().restricted(&Filter::ARTIFACT, SpendRider::None)
        ]),
        // NOT SUPPORTED: "{3}, {T}, Sacrifice an artifact: Create a 3/3
        // colorless Robot Villain artifact creature token named Doombot.
        // Activate only as a sorcery." — `Effect::CreateToken` names a
        // `TokenDef`, and the only definitions a card may reach are the
        // constants in `crate::tokens`, whose index in `ALL` is the token's
        // art key: a literal written here would compile, resolve and reach
        // the table with no identity at all
        // (`no_card_file_defines_its_own_token`). The registry holds no
        // Doombot, and adding one means editing another file, so the ability
        // comes off the card rather than shipping half of it.
    ],
);
