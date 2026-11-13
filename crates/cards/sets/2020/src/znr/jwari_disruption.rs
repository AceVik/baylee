//! Jwari Disruption // Jwari Ruins — {1}{U} — Instant // Land
//! Oracle: Counter target spell unless its controller pays {1}.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Set: ZNR #64 — Zendikar Rising | Scryfall ID: 301750a7-d1fd-435e-bfa8-9d2fb22ad627 | Oracle ID: 941a4b14-ea2a-4bd0-8cc2-d609f80df32c
//! Face: Jwari Disruption — {1}{U} — Instant
//! Face: Jwari Ruins —  — Land
// IMPLEMENTED — the counterspell as `PlayerMayPayOr` on the targeted spell's
// controller (pays {1}, or `CounterTargetSpell` runs); the back face carries
// `EnterModifier::Tapped` and its own {T}: Add {U} mana ability.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])];

card!(
    index = index::JWARI_DISRUPTION,
    oracle_id = "941a4b14-ea2a-4bd0-8cc2-d609f80df32c",
    scryfall_id = "301750a7-d1fd-435e-bfa8-9d2fb22ad627",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Jwari Disruption",
            mana_cost = mana!("{1}{U}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Jwari Ruins",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::Fixed(1),
            effect: &Effect::CounterTargetSpell,
        }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )],
);
