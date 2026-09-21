//! Suppression Ray // Orderly Plaza — {3}{W/U}{W/U} — Sorcery // Land
//! Oracle: Tap all creatures target player controls. You may pay any amount of {E}. If you do, choose up to that many creatures tapped this way. Put a stun counter on each of them. (If a permanent with a stun counter would become untapped, remove one from it instead.)
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W} or {U}.
//! Set: MH3 #260 — Modern Horizons 3 | Scryfall ID: 0cccd328-457a-48ab-97fb-4bc319db2e60 | Oracle ID: b592568b-11b0-4081-90a7-30cfb9c1ba80
//! Face: Suppression Ray — {3}{W/U}{W/U} — Sorcery
//! Face: Orderly Plaza —  — Land
// PARTIAL — Orderly Plaza is built in full: it enters tapped and taps for
// {W} or {U}. Suppression Ray cannot be written at all; see NOT SUPPORTED.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "Tap all creatures target player controls." — no effect sweeps a filter bounded by a chosen seat; `Effect::TapTarget` taps only the objects the caster names, and `Effect::PumpFilter` is the one effect carrying a `controlled_by`.
// NOT SUPPORTED: "You may pay any amount of {E}. If you do, choose up to that many creatures tapped this way. Put a stun counter on each of them." — no cost or effect pays energy, and a stun counter has no `CounterKind` (`baylee_cards_dsl::counters` assigns none), nor the rule that removes one instead of untapping.

card!(
    index = index::SUPPRESSION_RAY,
    oracle_id = "b592568b-11b0-4081-90a7-30cfb9c1ba80",
    scryfall_id = "0cccd328-457a-48ab-97fb-4bc319db2e60",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[
        face!(
            name = "Suppression Ray",
            mana_cost = mana!("{3}{W/U}{W/U}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Orderly Plaza",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = &[mana_ability!(&[Effect::mana_choice(&[
                ManaColor::White,
                ManaColor::Blue,
            ])])],
        ),
    ],
    coverage = Coverage::Partial(
        "Suppression Ray: nothing taps all creatures a chosen player controls, \
         {E} cannot be paid, and stun counters have no CounterKind",
    ),
);
