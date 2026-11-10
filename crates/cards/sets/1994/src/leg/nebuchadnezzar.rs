//! Nebuchadnezzar — {3}{U}{B} — Legendary Creature — Human Wizard
//! Oracle: {X}, {T}: Choose a card name. Target opponent reveals X cards at random from their hand. Then that player discards all cards with that name revealed this way. Activate only during your turn.
//! Set: ME3 #162 — Masters Edition III | Scryfall ID: 1e50afb3-cf9f-4ce3-91ff-84f99860c181 | Oracle ID: 7c48448e-4e13-4cda-999e-e06ae0f7dcea
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NEBUCHADNEZZAR,
    oracle_id = "7c48448e-4e13-4cda-999e-e06ae0f7dcea",
    scryfall_id = "1e50afb3-cf9f-4ce3-91ff-84f99860c181",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Nebuchadnezzar",
        mana_cost = mana!("{3}{U}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(3),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{X}", TapSelf),
        &[
            Effect::ChooseCardName {
                player: PlayerRel::You,
                types: TypeSet::EMPTY,
                excluded_types: TypeSet::EMPTY,
                remember: false
            },
            Effect::RevealRandomDiscardNamed {
                player: PlayerRel::Chosen,
                count: Amount::X
            },
        ],
        target = Some(TargetSpec::AnyOpponent),
        condition = Some(Condition::YourTurn)
    )],
);
