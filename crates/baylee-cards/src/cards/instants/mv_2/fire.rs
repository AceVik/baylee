//! Fire // Ice — {1}{R} — Instant // Instant
//! Oracle: Fire deals 2 damage divided as you choose among one or two targets.
//! Oracle: Tap target permanent.
//! Oracle: Draw a card.
//! Set: DMR #215 — Dominaria Remastered | Scryfall ID: 18303862-4726-4136-814f-157aa7006579 | Oracle ID: ae92942b-919c-4ea9-b693-85fcef765d5a
//! Face: Fire — {1}{R} — Instant
//! Face: Ice — {1}{U} — Instant
// PARTIAL — Ice's half is built in full (tap the target permanent, then draw
// a card); Fire's half is not, for the reason written at the NOT SUPPORTED
// note below.

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
        ),
        face!(
            name = "Ice",
            mana_cost = mana!("{1}{U}"),
            types = TypeSet::INSTANT,
            // "Tap target permanent. Draw a card." A `TargetSpec::Object` is
            // read on the battlefield, where every object is a permanent, so
            // the filter is the one that names all of them.
            abilities = &[spell!(
                &[Effect::TapTarget, Effect::draw(1)],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
            )],
        ),
    ],
    coverage = Coverage::Partial(
        "Fire: \"deals 2 damage divided as you choose among one or two targets\" \
         — nothing in the DSL divides an amount among targets"
    ),
    // NOT SUPPORTED: "Fire deals 2 damage divided as you choose among one or
    // two targets." `Effect::DealDamage` deals its whole amount to every
    // target it is handed, `TargetReq` carries no per-target allocation and no
    // `Amount` is divisible — so the front face keeps no spell ability rather
    // than one that deals 2 damage to each of two targets.
);
