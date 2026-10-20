//! Battle Cry — {2}{W} — Instant
//! Oracle: Untap all white creatures you control.
//! Oracle: Whenever a creature blocks this turn, it gets +0/+1 until end of turn.
//! Set: ICE #5 — Ice Age | Scryfall ID: c558a8c4-035c-464e-9ff8-c188c1bb619e | Oracle ID: b92afdba-5aea-42c8-bdd8-6ba5ff6cba4f

use baylee_cards_dsl::prelude::*;

static WHITE_CREATURES_YOU_CONTROL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
    Filter::ControlledByYou,
]);

// NOT SUPPORTED: "Whenever a creature blocks this turn, it gets +0/+1 until end of turn." —
// the DSL has no turn-scoped delayed trigger on any creature blocking that pumps the blocking creature.
card!(
    index = index::BATTLE_CRY,
    oracle_id = "b92afdba-5aea-42c8-bdd8-6ba5ff6cba4f",
    scryfall_id = "c558a8c4-035c-464e-9ff8-c188c1bb619e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage =
        Coverage::Partial("missing delayed trigger pumping each creature that blocks this turn"),
    faces = &[face!(
        name = "Battle Cry",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::UntapAll {
        filter: &WHITE_CREATURES_YOU_CONTROL,
    }])],
);
