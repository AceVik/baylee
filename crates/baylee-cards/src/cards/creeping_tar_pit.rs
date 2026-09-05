//! Creeping Tar Pit — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U} or {B}.
//! Oracle: {1}{U}{B}: Until end of turn, this land becomes a 3/2 blue and black Elemental creature. It's still a land. It can't be blocked this turn.
//! Set: CLB #888 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 1f60e172-fcdc-4699-a212-815201375d47 | Oracle ID: 250cb58b-2924-4dff-92fe-ac0ebbbeb218
// IMPLEMENTED — enters tapped, taps for {U} or {B}, animates into a 3/2 unblockable Elemental creature land.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 383,
    oracle_id: "250cb58b-2924-4dff-92fe-ac0ebbbeb218",
    scryfall_id: "1f60e172-fcdc-4699-a212-815201375d47",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Creeping Tar Pit",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::Tapped],
    },
    ],
    abilities: &[
        mana_ability!(
            Cost::TAP,
            &[Effect::mana_choice(&[
                ManaColor::Blue,
                ManaColor::Black,
            ])]
        ),
        activated!(
            Cost {
                mana: baylee_core::mana!("{1}{U}{B}"),
                parts: &[],
            },
            &[
                Effect::CreateContinuousEffect {
                    layer: Layer::Type,
                    filter: &Filter::This,
                    modifier: Modifier::AddType(TypeSet::CREATURE),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::CreateContinuousEffect {
                    layer: Layer::Type,
                    filter: &Filter::This,
                    modifier: Modifier::AddSubtype(subtypes::creature::ELEMENTAL),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::CreateContinuousEffect {
                    layer: Layer::Color,
                    filter: &Filter::This,
                    modifier: Modifier::SetColor(ColorSet::from_slice(&[
                        Color::Blue,
                        Color::Black,
                    ])),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::CreateContinuousEffect {
                    layer: Layer::Ability,
                    filter: &Filter::This,
                    modifier: Modifier::AddKeyword(KeywordSet::UNBLOCKABLE),
                    duration: Duration::UntilEndOfTurn,
                },
                Effect::CreateContinuousEffect {
                    layer: Layer::PtSet,
                    filter: &Filter::This,
                    modifier: Modifier::SetPT(3, 2),
                    duration: Duration::UntilEndOfTurn,
                },
            ],
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 383);
        assert_eq!(CARD.oracle_id, "250cb58b-2924-4dff-92fe-ac0ebbbeb218");
        assert_eq!(CARD.scryfall_id, "1f60e172-fcdc-4699-a212-815201375d47");
        assert_eq!(CARD.faces[0].name, "Creeping Tar Pit");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[0].enter_modifiers, &[EnterModifier::Tapped]);
        assert_eq!(
            CARD.color_identity,
            ColorSet::from_slice(&[Color::Black, Color::Blue])
        );
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}
