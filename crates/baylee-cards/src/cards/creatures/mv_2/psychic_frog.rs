//! Psychic Frog — {U}{B} — Creature — Frog
//! Oracle: Whenever this creature deals combat damage to a player or planeswalker, draw a card.
//! Oracle: Discard a card: Put a +1/+1 counter on this creature.
//! Oracle: Exile three cards from your graveyard: This creature gains flying until end of turn.
//! Set: MH3 #199 — Modern Horizons 3 | Scryfall ID: 68924203-c3d9-41ce-8ca8-c6dd491eb3ca | Oracle ID: e157ad1e-be35-47f3-92db-01799f8fb6a5
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PSYCHIC_FROG,
    oracle_id = "e157ad1e-be35-47f3-92db-01799f8fb6a5",
    scryfall_id = "68924203-c3d9-41ce-8ca8-c6dd491eb3ca",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Psychic Frog",
        mana_cost = mana!("{U}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FROG],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[
        triggered!(
            Trigger::DealsCombatDamageToPlayerOrPlaneswalker(&Filter::This),
            &[Effect::draw(1)],
        ),
        activated!(
            cost!(Discard(&Filter::Any)),
            &[Effect::AddCounterSource {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
        ),
        activated!(
            cost!(
                ExileFromGraveyard(&Filter::Any),
                ExileFromGraveyard(&Filter::Any),
                ExileFromGraveyard(&Filter::Any)
            ),
            &[Effect::ModifySource {
                modifier: Modifier::AddKeyword(KeywordSet::FLYING),
                duration: Duration::UntilEndOfTurn,
            }],
        ),
    ],
);
