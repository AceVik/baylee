//! Legion's Landing // Adanto, the First Fort — {W} — Legendary Enchantment // Legendary Land
//! Oracle: When Legion's Landing enters, create a 1/1 white Vampire creature token with lifelink.
//! Oracle: When you attack with three or more creatures, transform Legion's Landing.
//! Oracle: (Transforms from Legion's Landing.)
//! Oracle: {T}: Add {W}.
//! Oracle: {2}{W}, {T}: Create a 1/1 white Vampire creature token with lifelink.
//! Set: XLN #22 — Ixalan | Scryfall ID: 05e2a5e6-3aaa-4096-bdd0-fcc1afe5a36c | Oracle ID: f7d8b91b-6541-4d3e-af51-7e000eac69c1
//! Face: Legion's Landing — {W} — Legendary Enchantment
//! Face: Adanto, the First Fort —  — Legendary Land
// PARTIAL — built: the back face's `{T}: Add {W}`. Built nothing else; the
// three clauses that are not built each carry a NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "When you attack with three or more creatures, transform
// Legion's Landing" — no `Trigger` variant counts attackers. `Attacks(filter)`
// fires once per attacking object and says nothing about how many attacked,
// and no `Condition` counts attacking creatures, so "attack with three or
// more" has nothing to hang the transform on.
// NOT SUPPORTED: "When Legion's Landing enters, create a 1/1 white Vampire
// creature token with lifelink" — `Effect::CreateToken` needs a `&'static
// TokenDef`, and the pool's registry (`crate::tokens`) holds no 1/1 white
// Vampire with lifelink. A card file may not define a token of its own: the
// index into `crate::tokens::ALL` is the art key, so a local literal reaches
// the table nameless.
// NOT SUPPORTED: "{2}{W}, {T}: Create a 1/1 white Vampire creature token with
// lifelink" — the same absent token, so the ability comes off the card rather
// than shipping a cost that pays for nothing.
static BACK_ABILITIES: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])];

card!(
    index = index::LEGION_S_LANDING,
    oracle_id = "f7d8b91b-6541-4d3e-af51-7e000eac69c1",
    scryfall_id = "05e2a5e6-3aaa-4096-bdd0-fcc1afe5a36c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "no Trigger counts attacking creatures, so \"when you attack with three or more \
         creatures\" cannot be stated; and the pool has no 1/1 white Vampire token with \
         lifelink in crate::tokens, which both token-creating clauses need"
    ),
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
