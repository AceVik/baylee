//! Outmaneuver — {X}{R} — Instant
//! Oracle: X target blocked creatures assign their combat damage this turn as though they weren't blocked.
//! Set: USG #205 — Urza's Saga | Scryfall ID: 4a5a69fa-71ff-4e4f-9406-7cfebccb3384 | Oracle ID: 42b44108-4a03-4333-bf5a-e42493f4d5e4
// PARTIAL

use baylee_cards_dsl::prelude::*;

const BLOCKED_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Attacking,
    Filter::Not(&Filter::Unblocked),
]);

card!(
    index = index::OUTMANEUVER,
    oracle_id = "42b44108-4a03-4333-bf5a-e42493f4d5e4",
    scryfall_id = "4a5a69fa-71ff-4e4f-9406-7cfebccb3384",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "assigning combat damage as though unblocked is not in Modifier; targets X blocked attacking creatures and grants them trample until end of turn",
    ),
    faces = &[face!(
        name = "Outmaneuver",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::AddKeyword(KeywordSet::TRAMPLE),
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&BLOCKED_CREATURE))),
    )],
);
