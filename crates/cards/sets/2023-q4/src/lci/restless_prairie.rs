//! Restless Prairie — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G} or {W}.
//! Oracle: {2}{G}{W}: This land becomes a 3/3 green and white Llama creature until end of turn. It's still a land.
//! Oracle: Whenever this land attacks, other creatures you control get +1/+1 until end of turn.
//! Set: LCI #281 — The Lost Caverns of Ixalan | Scryfall ID: f94ef116-6aff-4f53-a7f9-be5e21c7afa4 | Oracle ID: c071257a-63e7-48d0-a677-0b396a09b624
// IMPLEMENTED — enters tapped; {T} for {G} or {W}; the animation is four
// continuous effects on the source (creature, Llama, green/white, 3/3) until
// end of turn, "it's still a land" being the printed type it keeps; the attack
// trigger pumps every other creature you control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::RESTLESS_PRAIRIE,
    oracle_id = "c071257a-63e7-48d0-a677-0b396a09b624",
    scryfall_id = "f94ef116-6aff-4f53-a7f9-be5e21c7afa4",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Restless Prairie",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Green, ManaColor::White])]),
        activated!(
            cost!("{2}{G}{W}"),
            &[Effect::Sequence(&[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::LLAMA),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetColor(ColorSet::from_slice(&[Color::Green, Color::White])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(3, 3),
                    Duration::UntilEndOfTurn,
                ),
            ])],
        ),
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::PumpFilter {
                filter: &Filter::ANOTHER_CREATURE_YOU_CONTROL,
                controlled_by: None,
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
        ),
    ],
);
