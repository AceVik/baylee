//! Ojer Kaslem, Deepest Growth // Temple of Cultivation — {3}{G}{G} — Legendary Creature — God // Land
//! Oracle: Trample
//! Oracle: Whenever Ojer Kaslem deals combat damage to a player, reveal that many cards from the top of your library. You may put a creature card and/or a land card from among them onto the battlefield. Put the rest on the bottom in a random order.
//! Oracle: When Ojer Kaslem dies, return it to the battlefield tapped and transformed under its owner's control.
//! Oracle: (Transforms from Ojer Kaslem, Deepest Growth.)
//! Oracle: {T}: Add {G}.
//! Oracle: {2}{G}, {T}: Transform this land. Activate only if you control ten or more permanents and only as a sorcery.
//! Set: LCI #204 — The Lost Caverns of Ixalan | Scryfall ID: 0cbc43a3-8cba-4988-9de1-c89aedd79ada | Oracle ID: eda11077-b2ce-408b-b982-def2da8fe599
//! Face: Ojer Kaslem, Deepest Growth — {3}{G}{G} — Legendary Creature — God
//! Face: Temple of Cultivation —  — Land
// IMPLEMENTED — trample; on combat damage to a player reveal that many cards and
// put a creature and/or a land from among them onto the battlefield; on dying
// it returns tapped and transformed; Temple of Cultivation taps for {G} and
// transforms back once its controller has ten permanents.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The back face's abilities: `{T}: Add {G}`, and the transform back once its
/// controller has ten permanents.
static TEMPLE_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
    // "Transform this land": the same permanent turns over (CR 701.27a) and
    // keeps its controller, its tapped state and whatever applied to it
    // (CR 712.18).
    activated!(
        cost!("{2}{G}", TapSelf),
        &[Effect::TransformSource],
        timing = ActivationTiming::SorcerySpeed,
        condition = Some(Condition::ControlCount(&Filter::Any, 10))
    ),
];

card!(
    index = index::OJER_KASLEM_DEEPEST_GROWTH,
    oracle_id = "eda11077-b2ce-408b-b982-def2da8fe599",
    scryfall_id = "0cbc43a3-8cba-4988-9de1-c89aedd79ada",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[
        face!(
            name = "Ojer Kaslem, Deepest Growth",
            mana_cost = mana!("{3}{G}{G}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::GOD],
            power = Some(6),
            toughness = Some(5),
            keywords = KeywordSet::TRAMPLE,
        ),
        face!(
            name = "Temple of Cultivation",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            abilities = TEMPLE_ABILITIES,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::This),
            &[Effect::RevealTopPutOntoBattlefield {
                count: Amount::EventAmount,
                picks: &[&Filter::CREATURE, &Filter::LAND],
            }]
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnSourceFromGraveyard {
                face: Some(1),
                tapped: true,
                owner_control: true,
                counters: None,
            }]
        ),
    ],
);
