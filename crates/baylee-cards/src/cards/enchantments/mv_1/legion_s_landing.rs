//! Legion's Landing // Adanto, the First Fort — {W} — Legendary Enchantment // Legendary Land
//! Oracle: When Legion's Landing enters, create a 1/1 white Vampire creature token with lifelink.
//! Oracle: When you attack with three or more creatures, transform Legion's Landing.
//! Oracle: (Transforms from Legion's Landing.)
//! Oracle: {T}: Add {W}.
//! Oracle: {2}{W}, {T}: Create a 1/1 white Vampire creature token with lifelink.
//! Set: XLN #22 — Ixalan | Scryfall ID: 05e2a5e6-3aaa-4096-bdd0-fcc1afe5a36c | Oracle ID: f7d8b91b-6541-4d3e-af51-7e000eac69c1
//! Face: Legion's Landing — {W} — Legendary Enchantment
//! Face: Adanto, the First Fort —  — Legendary Land
// PARTIAL — built: the enters Vampire and the back face's `{T}: Add {W}`.
// The transform is not, and the back face's Vampire waits for it; both carry
// a NOT SUPPORTED line below.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "When you attack with three or more creatures, transform
// Legion's Landing" — no `Trigger` fires once for a declaration of three or
// more attackers (`Attacks(filter)` fires once per attacker), and nothing
// transforms a permanent in place (#206). So Adanto is never reached.
// NOT SUPPORTED: "{2}{W}, {T}: Create a 1/1 white Vampire creature token with
// lifelink" — sayable (the same token as the enters trigger), and left off a
// face nothing reaches until #206, where no test could play it.
static BACK_ABILITIES: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])];

card!(
    index = index::LEGION_S_LANDING,
    oracle_id = "f7d8b91b-6541-4d3e-af51-7e000eac69c1",
    scryfall_id = "05e2a5e6-3aaa-4096-bdd0-fcc1afe5a36c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "\"When you attack with three or more creatures, transform Legion's Landing\" \
         cannot be stated: no Trigger fires once for a declaration of three or more \
         attackers (Trigger::Attacks fires per attacker) and nothing transforms a \
         permanent in place (#206), so Adanto, the First Fort is never reached, \
         and its {2}{W} Vampire ability is left unwritten with it"
    ),
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::CreateToken {
            token: &generated_tokens::VAMPIRE_1_1_WHITE_LIFELINK,
        }],
    )],
    faces = &[
        face!(
            name = "Legion's Landing",
            mana_cost = mana!("{W}"),
            types = TypeSet::ENCHANTMENT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Adanto, the First Fort",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            abilities = BACK_ABILITIES,
        ),
    ],
);
