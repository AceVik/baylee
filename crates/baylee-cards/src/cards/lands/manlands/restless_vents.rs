//! Restless Vents — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B} or {R}.
//! Oracle: {1}{B}{R}: Until end of turn, this land becomes a 2/3 black and red Insect creature with menace. It's still a land.
//! Oracle: Whenever this land attacks, you may discard a card. If you do, draw a card.
//! Set: LCI #284 — The Lost Caverns of Ixalan | Scryfall ID: e628e89b-bee9-408d-bb05-1784fda6b8a1 | Oracle ID: 696e7ddb-bdc7-40ee-bc5c-59e98f4a7401
// PARTIAL — enters tapped, the {B}/{R} mana ability and the {1}{B}{R}
// animation (creature type, Insect, black and red, 2/3, menace) are built.
// The attack trigger is NOT SUPPORTED — see the note beside its slot.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RESTLESS_VENTS,
    oracle_id = "696e7ddb-bdc7-40ee-bc5c-59e98f4a7401",
    scryfall_id = "e628e89b-bee9-408d-bb05-1784fda6b8a1",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Partial(
        "the attack trigger \"you may discard a card. If you do, draw a card\" has no \
         Effect that ties the draw to the discard — MayDo would draw on an empty hand",
    ),
    faces = &[face!(
        name = "Restless Vents",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Black, ManaColor::Red])]),
        activated!(
            cost!("{1}{B}{R}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(subtypes::creature::INSECT),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetColor(ColorSet::from_slice(&[Color::Black, Color::Red])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(2, 3),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::MENACE),
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
        // NOT SUPPORTED: "Whenever this land attacks, you may discard a card.
        // If you do, draw a card." — the printed "if you do" makes the draw
        // conditional on the discard, and the vocabulary has no effect that
        // reads a discard back: `MayDo { [DiscardForPlayers { You, 1 },
        // Effect::draw(1)] }` would draw for a player whose hand was empty
        // and who discarded nothing. The ability comes off the card rather
        // than shipping that.
    ],
);
