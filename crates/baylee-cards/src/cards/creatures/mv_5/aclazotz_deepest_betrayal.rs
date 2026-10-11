//! Aclazotz, Deepest Betrayal // Temple of the Dead — {3}{B}{B} — Legendary Creature — Bat God // Land
//! Oracle: Flying, lifelink
//! Oracle: Whenever Aclazotz attacks, each opponent discards a card. For each opponent who can't, you draw a card.
//! Oracle: Whenever an opponent discards a land card, create a 1/1 black Bat creature token with flying.
//! Oracle: When Aclazotz dies, return it to the battlefield tapped and transformed under its owner's control.
//! Oracle: (Transforms from Aclazotz, Deepest Betrayal.)
//! Oracle: {T}: Add {B}.
//! Oracle: {2}{B}, {T}: Transform this land. Activate only if a player has one or fewer cards in hand and only as a sorcery.
//! Set: LCI #88 — The Lost Caverns of Ixalan | Scryfall ID: 627c392c-4d18-4eb2-a4e8-c668f61f5487 | Oracle ID: fcdfe9d5-2743-4d3e-ab57-bf0f96beaa15
//! Face: Aclazotz, Deepest Betrayal — {3}{B}{B} — Legendary Creature — Bat God
//! Face: Temple of the Dead —  — Land
// IMPLEMENTED — attack, discard observer, death return and both faces.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Temple of the Dead's mana and conditional transformation.
static BACK_FACE_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
    activated!(
        cost!("{2}{B}", TapSelf),
        &[Effect::TransformSource],
        timing = ActivationTiming::SorcerySpeed,
        condition = Some(Condition::APlayerHandSizeAtMost(1))
    ),
];

card!(
    index = index::ACLAZOTZ_DEEPEST_BETRAYAL,
    oracle_id = "fcdfe9d5-2743-4d3e-ab57-bf0f96beaa15",
    scryfall_id = "627c392c-4d18-4eb2-a4e8-c668f61f5487",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLYING.union(KeywordSet::LIFELINK),
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Aclazotz, Deepest Betrayal",
            mana_cost = mana!("{3}{B}{B}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::BAT, subtypes::creature::GOD],
            power = Some(4),
            toughness = Some(4),
        ),
        face!(
            name = "Temple of the Dead",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            abilities = BACK_FACE_ABILITIES,
        ),
    ],
    abilities = &[
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::DiscardEachOrDraw {
                who: PlayerRel::EachOpponent
            }]
        ),
        triggered!(
            Trigger::Discarded {
                who: PlayerRel::EachOpponent,
                filter: &Filter::LAND
            },
            &[Effect::CreateToken {
                token: &crate::generated_tokens::BAT_1_1_BLACK_FLYING
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
