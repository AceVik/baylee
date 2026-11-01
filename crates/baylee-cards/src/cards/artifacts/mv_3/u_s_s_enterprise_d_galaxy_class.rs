//! U.S.S. Enterprise-D, Galaxy-Class — {3} — Legendary Artifact — Spacecraft
//! Oracle: Whenever one or more charge counters are put on U.S.S. Enterprise-D for the first time each turn, exile the top card of your library. You may play that card this turn.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 7+.)
//! Oracle: 7+ | Flying, vigilance
//! Set: TRK #273 — Star Trek | Scryfall ID: 057a4413-6a17-491e-bfd7-6cd427b1a442 | Oracle ID: d95af032-3efd-40c7-8229-ade9d974934f

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::U_S_S_ENTERPRISE_D_GALAXY_CLASS,
    oracle_id = "d95af032-3efd-40c7-8229-ade9d974934f",
    scryfall_id = "057a4413-6a17-491e-bfd7-6cd427b1a442",
    faces = &[face!(
        name = "U.S.S. Enterprise-D, Galaxy-Class",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(5),
    ),],
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    abilities = &[
        // Station (CR 702.184a): "Tap another untapped creature you
        // control: Put a number of charge counters on this permanent equal
        // to the tapped creature's power. Activate only as a sorcery." The
        // creature is a cost, not a target, and CR 118.3 is "untapped".
        activated!(
            cost!(TapOther(&Filter::ANOTHER_CREATURE_YOU_CONTROL)),
            &[Effect::AddCounter {
                kind: CounterKind::Charge,
                amount: Amount::TappedPower,
            }],
            timing = ActivationTiming::SorcerySpeed
        ),
        // "It's an artifact creature at 7+."
        static_ability!(
            Filter::This,
            Modifier::AddTypeIfCountersAtLeast {
                kind: CounterKind::Charge,
                at_least: 7,
                types: TypeSet::CREATURE,
            }
        ),
        // 7+ | Flying, vigilance
        static_ability!(
            Filter::This,
            Modifier::AddKeywordIfCountersAtLeast {
                kind: CounterKind::Charge,
                at_least: 7,
                keywords: KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
            }
        ),
        triggered!(
            Trigger::CountersPlaced {
                filter: &Filter::This,
                kind: CounterKind::Charge,
                by: PlayerRel::EachPlayer,
                first_this_turn: true
            },
            &[Effect::ExileTopToPlay {
                who: PlayerRel::You,
                play: ExilePlay {
                    count: 1,
                    window: PlayWindow::ThisTurn,
                    free: false,
                    mana_any_type: false,
                    spell_types: None
                }
            }],
        ),
    ],
);
