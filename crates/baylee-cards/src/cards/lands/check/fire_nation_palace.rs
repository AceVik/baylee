//! Fire Nation Palace — (no cost) — Land
//! Oracle: This land enters tapped unless you control a basic land.
//! Oracle: {T}: Add {R}.
//! Oracle: {1}{R}, {T}: Target creature you control gains firebending 4 until end of turn. (Whenever it attacks, add {R}{R}{R}{R}. This mana lasts until end of combat.)
//! Set: TLA #268 — Avatar: The Last Airbender | Scryfall ID: c91ac3f2-9fcd-4b41-a168-ae7f70b67d3c | Oracle ID: f2000fb8-39c6-4ad6-a020-5245faaa1eba
// PARTIAL — enters tapped unless you control a basic land (EnterModifier::TappedUnless);
// {T}: Add {R}. The firebending activation comes off the card (NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIRE_NATION_PALACE,
    oracle_id = "f2000fb8-39c6-4ad6-a020-5245faaa1eba",
    scryfall_id = "c91ac3f2-9fcd-4b41-a168-ae7f70b67d3c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Fire Nation Palace",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&Filter::YOUR_BASIC_LAND)],
    )],
    coverage = Coverage::Partial(
        "the {1}{R} activation: `firebending 4` is not a keyword the engine reads, and the \
         mana its trigger makes lasts until end of combat, which no mana effect can say",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        // NOT SUPPORTED: {1}{R}, {T}: Target creature you control gains firebending 4
        // until end of turn. (Whenever it attacks, add {R}{R}{R}{R}. This mana lasts
        // until end of combat.) — `firebending` is not one of the keyword bits the
        // engine reads, so neither a `keywords` entry, a `Modifier::AddKeyword` nor a
        // pump's keyword set could carry it; and `Effect::AddMana` has no duration
        // field to hold "this mana lasts until end of combat".
    ],
);
