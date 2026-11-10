//! Fire // Ice — {1}{R} — Instant // Instant
//! Oracle: Fire deals 2 damage divided as you choose among one or two targets.
//! Oracle: Tap target permanent.
//! Oracle: Draw a card.
//! Set: DMR #215 — Dominaria Remastered | Scryfall ID: 18303862-4726-4136-814f-157aa7006579 | Oracle ID: ae92942b-919c-4ea9-b693-85fcef765d5a
//! Face: Fire — {1}{R} — Instant
//! Face: Ice — {1}{U} — Instant

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIRE,
    oracle_id = "ae92942b-919c-4ea9-b693-85fcef765d5a",
    scryfall_id = "18303862-4726-4136-814f-157aa7006579",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[
        face!(
            name = "Fire",
            mana_cost = mana!("{1}{R}"),
            types = TypeSet::INSTANT,
            abilities = &[spell!(
                &[Effect::DealDamageDivided {
                    amount: DivisionAmount::Fixed(2)
                }],
                targets = Some(TargetReq {
                    spec: TargetSpec::AnyTarget,
                    min: 1,
                    max: 2,
                    count_is_x: false,
                    distinct_mana_values: false,
                    chooser: baylee_cards_dsl::TargetChooser::Controller,
                }),
            )],
        ),
        face!(
            name = "Ice",
            mana_cost = mana!("{1}{U}"),
            types = TypeSet::INSTANT,
            abilities = &[spell!(
                &[Effect::TapTarget, Effect::draw(1)],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
            )],
        ),
    ],
    coverage = Coverage::Implemented,
);
