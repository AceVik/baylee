//! Afterlife — {2}{W} — Instant
//! Oracle: Destroy target creature. It can't be regenerated. Its controller creates a 1/1 white Spirit creature token with flying.
//! Set: C14 #64 — Commander 2014 | Scryfall ID: 258946f2-294a-4ddf-be91-9ff4c94e4bdb | Oracle ID: 4c13e2b5-961a-4031-84b1-15bd19b94286
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AFTERLIFE,
    oracle_id = "4c13e2b5-961a-4031-84b1-15bd19b94286",
    scryfall_id = "258946f2-294a-4ddf-be91-9ff4c94e4bdb",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Afterlife",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::Destroy {
                target: TargetSpec::Object(&Filter::CREATURE),
                no_regen: true,
            },
            Effect::CreateTokenForTargetController {
                token: &crate::generated_tokens::SPIRIT_1_1_WHITE_FLYING,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
