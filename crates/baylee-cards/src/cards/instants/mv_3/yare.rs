//! Yare — {2}{W} — Instant
//! Oracle: Target creature defending player controls gets +3/+0 until end of turn. That creature can block up to two additional creatures this turn.
//! Set: MIR #48 — Mirage | Scryfall ID: 9a14343d-c2cd-4a67-ae54-6b6a4677ca85 | Oracle ID: f0f24c6d-80fb-4e99-a74d-6ed9d349ed3d
// IMPLEMENTED — target creature defending player controls gets +3/+0 until end of turn and can block up to two additional creatures this turn.

use baylee_cards_dsl::prelude::*;

static DEFENDING_PLAYERS_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::ControlledByDefendingPlayer]);

card!(
    index = index::YARE,
    oracle_id = "f0f24c6d-80fb-4e99-a74d-6ed9d349ed3d",
    scryfall_id = "9a14343d-c2cd-4a67-ae54-6b6a4677ca85",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Yare",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::PumpTarget {
                power: Amount::Fixed(3),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifyTargets {
                modifier: Modifier::CanBlockAdditional(2),
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &DEFENDING_PLAYERS_CREATURE
        ))),
    )],
);
