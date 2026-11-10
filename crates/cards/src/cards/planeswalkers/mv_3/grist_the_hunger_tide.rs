//! Grist, the Hunger Tide — {1}{B}{G} — Legendary Planeswalker — Grist
//! Oracle: As long as Grist isn't on the battlefield, it's a 1/1 Insect creature in addition to its other types.
//! Oracle: +1: Create a 1/1 black and green Insect creature token, then mill a card. If an Insect card was milled this way, put a loyalty counter on Grist and repeat this process.
//! Oracle: −2: You may sacrifice a creature. When you do, destroy target creature or planeswalker.
//! Oracle: −5: Each opponent loses life equal to the number of creature cards in your graveyard.
//! Set: DSC #220 — Duskmourn: House of Horror Commander | Scryfall ID: 1925dc45-4dee-4772-aa16-3b4ca54be6c7 | Oracle ID: 0efb0d7e-dea0-4817-a243-15066e9ef333
// IMPLEMENTED — the 1/1 Insect creature it is everywhere but the battlefield,
// +1 (Insect token, mill, repeat on a milled Insect card), −2 (optional
// sacrifice, then a reflexive destroy) and −5 (life loss for creature cards).

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "an Insect card" milled this way — read once Grist's own static has made it
/// one, off the battlefield.
const INSECT_CARD: Filter = Filter::HasSubtype(subtypes::creature::INSECT);

/// "If an Insect card was milled this way": what the loop reads after each
/// pass, for the loyalty counter and for going round again.
const INSECT_MILLED: Condition =
    Condition::AmountAtLeast(Amount::ThisWay(ThisWay::Milled(&INSECT_CARD)), 1);

card!(
    index = index::GRIST_THE_HUNGER_TIDE,
    oracle_id = "0efb0d7e-dea0-4817-a243-15066e9ef333",
    scryfall_id = "1925dc45-4dee-4772-aa16-3b4ca54be6c7",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    // The static below works outside the game too (CR 113.6c), so before the
    // game Grist is a legendary creature card, and a legendary creature card
    // may be a commander (CR 903.3).
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Grist, the Hunger Tide",
        mana_cost = mana!("{1}{B}{G}"),
        types = TypeSet::PLANESWALKER,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::planeswalker::GRIST],
        loyalty = Some(3),
    ),],
    abilities = &[
        // CR 113.6c: "as long as Grist isn't on the battlefield, it's a 1/1
        // Insect creature in addition to its other types" — one static per
        // layer, each working in every zone but the battlefield.
        static_ability!(
            Filter::This,
            Modifier::AddType(TypeSet::CREATURE),
            zone = ZoneRef::NotBattlefield
        ),
        static_ability!(
            Filter::This,
            Modifier::AddSubtype(subtypes::creature::INSECT),
            zone = ZoneRef::NotBattlefield
        ),
        static_ability!(
            Filter::This,
            Modifier::SetPT(1, 1),
            zone = ZoneRef::NotBattlefield
        ),
        loyalty!(
            1,
            &[Effect::RepeatWhile {
                effects: &[
                    Effect::CreateToken {
                        token: &generated_tokens::INSECT_1_1_BLACK_GREEN,
                    },
                    Effect::Mill {
                        amount: Amount::Fixed(1),
                        target: PlayerRel::You,
                    },
                    Effect::IfCondition {
                        condition: INSECT_MILLED,
                        then: &[Effect::AddCounterSource {
                            kind: CounterKind::Loyalty,
                            amount: Amount::Fixed(1),
                        }],
                        otherwise: &[],
                    },
                ],
                condition: INSECT_MILLED,
            }]
        ),
        // "You may sacrifice a creature. When you do, destroy target creature
        // or planeswalker." The target is chosen only when the reflexive
        // trigger goes on the stack, after the sacrifice.
        loyalty!(
            -2,
            &[
                Effect::MayDo {
                    effects: &[Effect::SacrificeFilter {
                        who: PlayerRel::You,
                        filter: &Filter::CREATURE,
                    }],
                },
                Effect::Reflexive {
                    when: ReflexiveEvent::Sacrificed(&Filter::CREATURE),
                    effects: &[Effect::destroy(TargetSpec::Object(
                        &Filter::CREATURE_OR_PLANESWALKER
                    ))],
                    target: Some(TargetSpec::Object(&Filter::CREATURE_OR_PLANESWALKER)),
                },
            ]
        ),
        loyalty!(
            -5,
            &[Effect::LoseLife {
                amount: Amount::CountOf {
                    filter: &Filter::CREATURE,
                    zone: ZoneSel::GraveyardYou,
                },
                target: PlayerRel::EachOpponent,
            }]
        ),
    ],
);
