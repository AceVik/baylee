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
// PARTIAL — trample, the back face's {T}: Add {G} and its transform ability are
// written; the front face's combat-damage clause and its dies trigger are not.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The back face's abilities: `{T}: Add {G}`, and the transform back once its
/// controller has ten permanents.
static TEMPLE_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
    activated!(
        cost!("{2}{G}", TapSelf),
        &[Effect::ExileSelfReturnAsFace { face: 0 }],
        timing = ActivationTiming::SorcerySpeed,
        condition = Some(Condition::ControlCount(&Filter::Any, 10))
    ),
];

// NOT SUPPORTED: "Whenever Ojer Kaslem deals combat damage to a player, reveal
// that many cards from the top of your library. You may put a creature card
// and/or a land card from among them onto the battlefield. Put the rest on the
// bottom in a random order." — `Trigger::DealsCombatDamageToPlayer` says the
// trigger, but no `Effect` reveals a counted number of cards and then takes a
// creature card and/or a land card from among them onto the battlefield;
// `LookAtTopPick` keeps what it picks in the hand and bottoms the rest.
// NOT SUPPORTED: "When Ojer Kaslem dies, return it to the battlefield tapped
// and transformed under its owner's control." — `Effect::ExileSelfReturnAsFace`
// exiles a source that is still on the battlefield and carries no "tapped", so
// nothing returns a card that has already died as its other face.

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
            types = TypeSet::LAND,
            abilities = TEMPLE_ABILITIES,
        ),
    ],
    coverage = Coverage::Partial(
        "the combat-damage reveal clause and the dies trigger have no DSL effect"
    ),
);
