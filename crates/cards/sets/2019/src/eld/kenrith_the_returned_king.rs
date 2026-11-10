//! Kenrith, the Returned King — {4}{W} — Legendary Creature — Human Noble
//! Oracle: {R}: All creatures gain trample and haste until end of turn.
//! Oracle: {1}{G}: Put a +1/+1 counter on target creature.
//! Oracle: {2}{W}: Target player gains 5 life.
//! Oracle: {3}{U}: Target player draws a card.
//! Oracle: {4}{B}: Put target creature card from a graveyard onto the battlefield under its owner's control.
//! Set: PLST #ELD-303 — The List | Scryfall ID: 0e259db1-14db-4314-998c-6a076a28d8cb | Oracle ID: d209b948-9afb-4fd1-a961-72c87282878c

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "target creature card from a graveyard" — no player named, so any
/// graveyard at the table.
const ANY_GRAVEYARD_CREATURE: TargetSpec =
    TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::EachPlayer);

card!(
    index = index::KENRITH_THE_RETURNED_KING,
    oracle_id = "d209b948-9afb-4fd1-a961-72c87282878c",
    scryfall_id = "0e259db1-14db-4314-998c-6a076a28d8cb",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White
    ]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Kenrith, the Returned King",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::NOBLE],
        power = Some(5),
        toughness = Some(5),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        activated!(
            cost!("{R}"),
            &[Effect::PumpFilter {
                filter: &Filter::CREATURE,
                controlled_by: None,
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::TRAMPLE.union(KeywordSet::HASTE),
                duration: Duration::UntilEndOfTurn,
            }]
        ),
        activated!(
            cost!("{1}{G}"),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE))
        ),
        activated!(
            cost!("{2}{W}"),
            &[Effect::GainLifeFor {
                amount: Amount::Fixed(5),
                who: PlayerRel::Chosen,
            }],
            target = Some(TargetSpec::AnyPlayer)
        ),
        activated!(
            cost!("{3}{U}"),
            &[Effect::DrawCardsFor {
                amount: Amount::Fixed(1),
                who: PlayerRel::Chosen,
            }],
            target = Some(TargetSpec::AnyPlayer)
        ),
        // "…onto the battlefield under its owner's control": a creature card
        // from an opponent's graveyard comes back on their side.
        activated!(
            cost!("{4}{B}"),
            &[Effect::GraveyardToBattlefield {
                target: ANY_GRAVEYARD_CREATURE,
                owner_control: true,
                counters: None,
            }],
            target = Some(ANY_GRAVEYARD_CREATURE)
        ),
    ],
);
