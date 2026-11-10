//! Foxfire — {2}{G} — Instant
//! Oracle: Untap target attacking creature. Prevent all combat damage that would be dealt to and dealt by that creature this turn.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: 5ED #295 — Fifth Edition | Scryfall ID: eb1638c4-effb-460b-af5d-aa17559dbd37 | Oracle ID: 6c270e6a-12a7-4323-87ba-6d770986fb48
use baylee_cards_dsl::prelude::*;

card!(
    index = index::FOXFIRE,
    oracle_id = "6c270e6a-12a7-4323-87ba-6d770986fb48",
    scryfall_id = "eb1638c4-effb-460b-af5d-aa17559dbd37",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Foxfire",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::UntapTarget,
            Effect::ModifyTargets {
                modifier: Modifier::PreventDamageToIt,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifyTargets {
                modifier: Modifier::PreventDamageFromIt,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::ATTACKING_CREATURE
        ))),
    )],
);
