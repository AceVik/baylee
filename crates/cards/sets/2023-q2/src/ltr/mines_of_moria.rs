//! Mines of Moria — (no cost) — Legendary Land
//! Oracle: Mines of Moria enters tapped unless you control a legendary creature.
//! Oracle: {T}: Add {R}.
//! Oracle: {3}{R}, {T}, Exile three cards from your graveyard: Create two Treasure tokens.
//! Set: LTR #257 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: 0be723d6-4ada-4c3f-b87b-8ab83a4bbb8f | Oracle ID: 583cdebe-0195-45be-bd2e-5765f07cb902

use crate::tokens::TREASURE;
use baylee_cards_dsl::prelude::*;

static LEGENDARY_CREATURE_YOU_CONTROL: Filter = f!(your LEGENDARY_CREATURE);

card!(
    index = index::MINES_OF_MORIA,
    oracle_id = "583cdebe-0195-45be-bd2e-5765f07cb902",
    scryfall_id = "0be723d6-4ada-4c3f-b87b-8ab83a4bbb8f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mines of Moria",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
        enter_modifiers = &[EnterModifier::TappedUnless(&LEGENDARY_CREATURE_YOU_CONTROL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        // Three cards are three parts, one question each, the way Time
        // Sieve writes its five sacrifices.
        activated!(
            cost!(
                "{3}{R}",
                TapSelf,
                ExileFromGraveyard(&Filter::Any),
                ExileFromGraveyard(&Filter::Any),
                ExileFromGraveyard(&Filter::Any)
            ),
            &[Effect::CreateTokenN {
                token: &TREASURE,
                amount: Amount::Fixed(2),
            }]
        ),
    ],
);
