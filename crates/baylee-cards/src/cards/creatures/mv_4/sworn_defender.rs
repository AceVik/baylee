//! Sworn Defender — {2}{W}{W} — Creature — Human Knight
//! Oracle: {1}: This creature's power becomes the toughness of target creature blocking or being blocked by this creature minus 1 until end of turn, and its toughness becomes 1 plus the power of that creature until end of turn.
//! Set: ALL #19 — Alliances | Scryfall ID: 328e6ceb-30f7-415e-93b4-7075af0fed89 | Oracle ID: 02197d14-8d9f-4b1f-9ee0-ce8ef690843c

// PARTIAL — {1}: this creature's toughness becomes 1 plus the power of target creature blocking or being blocked by it, until end of turn.
// NOT SUPPORTED: "This creature's power becomes the toughness of that creature minus 1 until end of turn." Only toughness can be set from another creature's value (SetSourceToughness); Modifier::SetPower takes a constant, so the power stays printed (1).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLOCKING_OR_BLOCKED_BY_THIS: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[Filter::BlockingSource, Filter::BlockedBySource]),
]);

card!(
    index = index::SWORN_DEFENDER,
    oracle_id = "02197d14-8d9f-4b1f-9ee0-ce8ef690843c",
    scryfall_id = "328e6ceb-30f7-415e-93b4-7075af0fed89",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sworn Defender",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(1),
        toughness = Some(3),
    ),],
    coverage = Coverage::Partial(
        "The {1} ability sets this creature's base toughness to 1 plus the power of target creature blocking or being blocked by it until end of turn, without setting this creature's base power to that creature's toughness minus 1."
    ),
    abilities = &[activated!(
        cost!("{1}"),
        &[Effect::SetSourceToughness {
            base: Amount::TargetPower,
            offset: 1,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&BLOCKING_OR_BLOCKED_BY_THIS))
    )],
);
