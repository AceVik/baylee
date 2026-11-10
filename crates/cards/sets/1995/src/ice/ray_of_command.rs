//! Ray of Command — {3}{U} — Instant
//! Oracle: Untap target creature an opponent controls and gain control of it until end of turn. That creature gains haste until end of turn. When you lose control of the creature, tap it.
//! Set: DDM #29 — Duel Decks: Jace vs. Vraska | Scryfall ID: 7ca8a427-5c20-4650-bbb5-9895e4f2c009 | Oracle ID: adf41d29-403d-4936-9689-3148d103e700
use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAY_OF_COMMAND,
    oracle_id = "adf41d29-403d-4936-9689-3148d103e700",
    scryfall_id = "7ca8a427-5c20-4650-bbb5-9895e4f2c009",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ray of Command",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::UntapTarget,
            Effect::continuous(
                &Filter::This,
                Modifier::GainControl,
                Duration::UntilEndOfTurn,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::HASTE),
                Duration::UntilEndOfTurn,
            ),
            Effect::CreateDelayedObserver {
                watch: TargetSpec::AnyTarget,
                when: DelayedObservation::YouLoseControl,
                about: TargetSpec::AnyTarget,
                effects: &[Effect::TapEventObject],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::OPPONENT_CREATURE
        )))
    )],
);
