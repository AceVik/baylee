//! Creeping Tar Pit — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U} or {B}.
//! Oracle: {1}{U}{B}: Until end of turn, this land becomes a 3/2 blue and black Elemental creature. It's still a land. It can't be blocked this turn.
//! Set: CLB #888 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 1f60e172-fcdc-4699-a212-815201375d47 | Oracle ID: 250cb58b-2924-4dff-92fe-ac0ebbbeb218
// IMPLEMENTED — enters tapped, taps for {U} or {B}, and the manland
// animation as five until-end-of-turn effects on the source: creature,
// Elemental, blue and black, 3/2, and unblockable.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CREEPING_TAR_PIT,
    oracle_id = "250cb58b-2924-4dff-92fe-ac0ebbbeb218",
    scryfall_id = "1f60e172-fcdc-4699-a212-815201375d47",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Creeping Tar Pit",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Blue, ManaColor::Black])]),
        // "…becomes a 3/2 blue and black Elemental creature. It's still a
        // land. It can't be blocked this turn." — layer 4 adds the creature
        // type, layer 6 the Elemental subtype and the keyword, layer 5 the
        // two colours, layer 7b the 3/2. The land type is never removed, and
        // no part of the sentence targets (CR 115.1c), so every filter here
        // reads the source.
        activated!(
            cost!("{1}{U}{B}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(subtypes::creature::ELEMENTAL),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddColor(ColorSet::from_slice(&[Color::Blue, Color::Black])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(3, 2),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::UNBLOCKABLE),
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
    ],
);
