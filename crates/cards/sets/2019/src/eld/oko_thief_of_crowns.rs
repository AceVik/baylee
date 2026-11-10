//! Oko, Thief of Crowns — {1}{G}{U} — Legendary Planeswalker — Oko
//! Oracle: +2: Create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
//! Oracle: +1: Target artifact or creature loses all abilities and becomes a green Elk creature with base power and toughness 3/3.
//! Oracle: −5: Exchange control of target artifact or creature you control and target creature an opponent controls with power 3 or less.
//! Set: ELD #197 — Throne of Eldraine | Scryfall ID: 3462a3d0-5552-49fa-9eb7-100960c55891 | Oracle ID: 60c60923-ff1b-43f7-8768-731499fcffc9

use crate::tokens::FOOD;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OKO_THIEF_OF_CROWNS,
    oracle_id = "60c60923-ff1b-43f7-8768-731499fcffc9",
    scryfall_id = "3462a3d0-5552-49fa-9eb7-100960c55891",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Oko, Thief of Crowns",
        mana_cost = mana!("{1}{G}{U}"),
        types = TypeSet::PLANESWALKER,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::planeswalker::OKO],
        loyalty = Some(4),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        // +2: Create a Food token.
        loyalty!(2, &[Effect::CreateToken { token: &FOOD }]),
        // +1: one sentence, four layers (CR 613.1d–f, 613.4b), all with the
        // same timestamp and for good. The ruling keeps supertypes and drops
        // every other card type and every creature type.
        loyalty!(
            1,
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::LoseAllAbilities,
                    Duration::Indefinitely,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::BecomeType {
                        types: TypeSet::CREATURE,
                        subtype: subtypes::creature::ELK,
                    },
                    Duration::Indefinitely,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetColor(ColorSet::of(Color::Green)),
                    Duration::Indefinitely,
                ),
                Effect::continuous(&Filter::This, Modifier::SetPT(3, 3), Duration::Indefinitely),
            ],
            targets = Some(TargetReq::one(TargetSpec::Object(
                &Filter::ARTIFACT_OR_CREATURE
            ))),
        ),
        // −5: two instances of "target", exchanged (CR 701.12b).
        loyalty!(
            -5,
            &[Effect::ExchangeControl],
            targets = Some(TargetReq::one(TargetSpec::Object(
                &f!(your ARTIFACT_OR_CREATURE)
            ))),
            second_targets = Some(TargetReq::one(TargetSpec::Object(&Filter::And(&[
                Filter::OPPONENT_CREATURE,
                Filter::PowerAtMost(3),
            ])))),
        ),
    ],
);
