//! Restless Vinestalk — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G} or {U}.
//! Oracle: {3}{G}{U}: Until end of turn, this land becomes a 5/5 green and blue Plant creature with trample. It's still a land.
//! Oracle: Whenever this land attacks, up to one other target creature has base power and toughness 3/3 until end of turn.
//! Set: WOE #261 — Wilds of Eldraine | Scryfall ID: e5f3161d-3f69-4b06-ab73-c31fc0c1520c | Oracle ID: 0935faa2-fb90-48db-8a92-906ba0f374c7
// IMPLEMENTED — enters tapped, {G}/{U} mana, the animation as five until-EOT
// continuous effects (type/colour/keyword/P-T), and the attack trigger's
// set-to-3/3 on up to one other target creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RESTLESS_VINESTALK,
    oracle_id = "0935faa2-fb90-48db-8a92-906ba0f374c7",
    scryfall_id = "e5f3161d-3f69-4b06-ab73-c31fc0c1520c",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Restless Vinestalk",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Green, ManaColor::Blue])]),
        activated!(
            cost!("{3}{G}{U}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(subtypes::creature::PLANT),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddColor(ColorSet::from_slice(&[Color::Green, Color::Blue])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::TRAMPLE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(5, 5),
                    Duration::UntilEndOfTurn,
                ),
            ]
        ),
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::continuous(
                &Filter::This,
                Modifier::SetPT(3, 3),
                Duration::UntilEndOfTurn,
            )],
            targets = Some(TargetReq::up_to_one(TargetSpec::Object(
                &Filter::ANOTHER_CREATURE
            )))
        ),
    ],
);
