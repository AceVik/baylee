//! Disciple of Freyalise // Garden of Freyalise — {3}{G}{G}{G} — Creature — Elf Druid // Land
//! Oracle: When this creature enters, you may sacrifice another creature. If you do, you gain X life and draw X cards, where X is that creature's power.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {G}.
//! Set: MH3 #250 — Modern Horizons 3 | Scryfall ID: a8e9ea5a-5e10-4b77-baef-0352ff035483 | Oracle ID: 2699005b-a471-429f-a9d8-fbf2077ee2fd
//! Face: Disciple of Freyalise — {3}{G}{G}{G} — Creature — Elf Druid
//! Face: Garden of Freyalise —  — Land
// PARTIAL — the land face is exact: the shock-land entry
// (EnterModifier::TappedOrPayLife) and {T}: Add {G}. The creature's enter
// trigger is dropped; see the NOT SUPPORTED line at the foot of the file.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DISCIPLE_OF_FREYALISE,
    oracle_id = "2699005b-a471-429f-a9d8-fbf2077ee2fd",
    scryfall_id = "a8e9ea5a-5e10-4b77-baef-0352ff035483",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Disciple of Freyalise",
            mana_cost = mana!("{3}{G}{G}{G}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::ELF, subtypes::creature::DRUID],
            power = Some(3),
            toughness = Some(3),
        ),
        face!(
            name = "Garden of Freyalise",
            types = TypeSet::LAND,
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
        ),
    ],
    coverage = Coverage::Partial(
        "Disciple of Freyalise's enter trigger needs an amount equal to the \
         power of the creature it sacrifices, which no Amount reaches",
    ),
);

// NOT SUPPORTED: "When this creature enters, you may sacrifice another
// creature. If you do, you gain X life and draw X cards, where X is that
// creature's power." — the trigger targets nothing, so Amount::TargetPower
// has nothing to read it off, and Amount carries no variant for the power of
// a permanent Effect::SacrificeFilter (or a MayDo around it) removed. The
// ability is off the card rather than approximated: a MayDo wrapping
// SacrificeFilter would eat a creature and then resolve GainLife and
// DrawCards at zero, which is a card that plays and is simply wrong.
