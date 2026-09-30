//! Time Vault — {2} — Artifact
//! Oracle: This artifact enters tapped.
//! Oracle: This artifact doesn't untap during your untap step.
//! Oracle: If you would begin your turn while this artifact is tapped, you may skip that turn instead. If you do, untap this artifact.
//! Oracle: {T}: Take an extra turn after this one.
//! Set: VMA #287 — Vintage Masters | Scryfall ID: c367ffc1-8084-45a1-87d5-22183604d1cb | Oracle ID: 99d4d99d-cf56-45aa-aa39-a250695612f2
// PARTIAL — skipping a turn to untap it is not in the engine; it enters
// tapped, does not untap and takes an extra turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TIME_VAULT,
    oracle_id = "99d4d99d-cf56-45aa-aa39-a250695612f2",
    scryfall_id = "c367ffc1-8084-45a1-87d5-22183604d1cb",
    coverage = Coverage::Partial(
        "skipping a turn to untap it is not in the engine; it enters tapped, does not untap and takes an extra turn"
    ),
    faces = &[face!(
        name = "Time Vault",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::DoesNotUntap),
        activated!(Cost::TAP, &[Effect::TakeExtraTurn]),
        // NOT SUPPORTED: If you would begin your turn while this artifact is tapped, you
        // may skip that turn instead. If you do, untap this artifact.
    ],
);
