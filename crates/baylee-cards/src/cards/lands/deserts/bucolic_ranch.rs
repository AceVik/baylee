//! Bucolic Ranch — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a Mount spell.
//! Oracle: {3}, {T}: Look at the top card of your library. If it's a Mount card, you may reveal it and put it into your hand. If you don't put it into your hand, you may put it on the bottom of your library.
//! Set: OTJ #265 — Outlaws of Thunder Junction | Scryfall ID: 6c4f6b81-53d0-49fb-b404-c2ad67de7493 | Oracle ID: 8f5902bf-4bc4-4d0c-84ea-a425307a4eb2
// PARTIAL — {C} and the any-colour mana restricted to Mount spells; the
// {3}, {T} look clause has no variant (NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "a Mount spell" — what the restricted mana may be spent on.
static MOUNT_SPELL: Filter = Filter::HasSubtype(subtypes::creature::MOUNT);

card!(
    index = index::BUCOLIC_RANCH,
    oracle_id = "8f5902bf-4bc4-4d0c-84ea-a425307a4eb2",
    scryfall_id = "6c4f6b81-53d0-49fb-b404-c2ad67de7493",
    faces = &[face!(
        name = "Bucolic Ranch",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Partial(
        "{3}, {T}: no effect looks at the top card and then offers a conditional keep-or-bottom",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_of_any_color().restricted(&MOUNT_SPELL, SpendRider::None)]),
        // NOT SUPPORTED: "{3}, {T}: Look at the top card of your library. If
        // it's a Mount card, you may reveal it and put it into your hand. If
        // you don't put it into your hand, you may put it on the bottom of
        // your library." — LookAtTopPick keeps any `pick` of the `count`
        // cards looked at and consults no filter, RearrangeTopLibrary only
        // reorders, and BottomCardFromHand is about a hand.
    ],
);
