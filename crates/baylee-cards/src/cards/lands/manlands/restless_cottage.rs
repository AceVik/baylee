//! Restless Cottage — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B} or {G}.
//! Oracle: {2}{B}{G}: This land becomes a 4/4 black and green Horror creature until end of turn. It's still a land.
//! Oracle: Whenever this land attacks, create a Food token and exile up to one target card from a graveyard.
//! Set: WOE #258 — Wilds of Eldraine | Scryfall ID: 787eadf3-5005-4ae5-820f-4012a4d4e1a5 | Oracle ID: 7e16595f-bdeb-422e-b99a-bfc0ed52e9f8
// IMPLEMENTED — enters tapped; {T} for {B} or {G}; the {2}{B}{G} animation adds
// the creature type, Horror, its colours and 4/4 until end of turn while it
// stays a land; attacking makes a Food token and exiles up to one target card
// from any graveyard.

use crate::tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::RESTLESS_COTTAGE,
    oracle_id = "7e16595f-bdeb-422e-b99a-bfc0ed52e9f8",
    scryfall_id = "787eadf3-5005-4ae5-820f-4012a4d4e1a5",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Restless Cottage",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Black, ManaColor::Green])]),
        activated!(
            cost!("{2}{B}{G}"),
            &[Effect::Sequence(&[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::HORROR),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetColor(ColorSet::from_slice(&[Color::Black, Color::Green])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(4, 4),
                    Duration::UntilEndOfTurn,
                ),
            ])]
        ),
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[
                Effect::CreateToken {
                    token: &tokens::FOOD,
                },
                Effect::exile(TargetSpec::CardInGraveyard(
                    &Filter::Any,
                    PlayerRel::EachPlayer,
                )),
            ],
            targets = Some(TargetReq::up_to_one(TargetSpec::CardInGraveyard(
                &Filter::Any,
                PlayerRel::EachPlayer,
            ))),
        ),
    ],
);
