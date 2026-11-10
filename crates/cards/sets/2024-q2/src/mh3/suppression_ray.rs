//! Suppression Ray // Orderly Plaza — {3}{W/U}{W/U} — Sorcery // Land
//! Oracle: Tap all creatures target player controls. You may pay any amount of {E}. If you do, choose up to that many creatures tapped this way. Put a stun counter on each of them. (If a permanent with a stun counter would become untapped, remove one from it instead.)
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W} or {U}.
//! Set: MH3 #260 — Modern Horizons 3 | Scryfall ID: 0cccd328-457a-48ab-97fb-4bc319db2e60 | Oracle ID: b592568b-11b0-4081-90a7-30cfb9c1ba80
//! Face: Suppression Ray — {3}{W/U}{W/U} — Sorcery
//! Face: Orderly Plaza —  — Land
// IMPLEMENTED — only creatures actually tapped by the spell can receive stun counters.

use baylee_cards_dsl::prelude::*;

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
            abilities = &[spell!(
                &[Effect::TapAndPayEnergyForCounters {
                    who: PlayerRel::Chosen,
                    filter: &Filter::CREATURE,
                    kind: CounterKind::Stun,
                }],
                targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
            )],
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
    coverage = Coverage::Implemented,
);
