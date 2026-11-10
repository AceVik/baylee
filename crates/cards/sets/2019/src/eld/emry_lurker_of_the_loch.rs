//! Emry, Lurker of the Loch — {2}{U} — Legendary Creature — Merfolk Wizard
//! Oracle: Affinity for artifacts (This spell costs {1} less to cast for each artifact you control.)
//! Oracle: When Emry enters, mill four cards.
//! Oracle: {T}: Choose target artifact card in your graveyard. You may cast that card this turn. (You still pay its costs. Timing rules still apply.)
//! Set: EOC #71 — Edge of Eternities Commander | Scryfall ID: c977d89a-bfd1-4e98-9d95-3e41c53dd188 | Oracle ID: da3e7d3d-2ca0-40c3-9602-fca37c92f507
// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EMRY_LURKER_OF_THE_LOCH,
    oracle_id = "da3e7d3d-2ca0-40c3-9602-fca37c92f507",
    scryfall_id = "c977d89a-bfd1-4e98-9d95-3e41c53dd188",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Emry, Lurker of the Loch",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
        cost_reduction = Some(CostReduction::PerCount {
            amount: Amount::CountOf {
                filter: &Filter::YOUR_ARTIFACT,
                zone: ZoneSel::Battlefield
            },
            each: 1,
        }),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::Mill {
                amount: Amount::Fixed(4),
                target: PlayerRel::You,
            }]
        ),
        activated!(
            Cost::TAP,
            &[Effect::continuous(
                &Filter::This,
                Modifier::CastSpellsFromGraveyard,
                Duration::UntilEndOfTurn,
            )],
            target = Some(TargetSpec::CardInGraveyard(
                &Filter::ARTIFACT,
                PlayerRel::You
            ))
        ),
    ],
);
