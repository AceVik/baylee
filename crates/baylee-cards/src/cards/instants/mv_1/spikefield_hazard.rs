//! Spikefield Hazard // Spikefield Cave — {R} — Instant // Land
//! Oracle: Spikefield Hazard deals 1 damage to any target. If a permanent dealt damage this way would die this turn, exile it instead.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Set: ZNR #166 — Zendikar Rising | Scryfall ID: a69541db-3f4e-412f-aa8e-dec1e74f74dc | Oracle ID: 81036c9f-fe0a-45a7-bcd5-0d344f31055a
//! Face: Spikefield Hazard — {R} — Instant
//! Face: Spikefield Cave —  — Land
// IMPLEMENTED — front face: 1 damage to any target; back face: enters
// tapped and taps for {R}. The front face's exile rider is not expressible.

use baylee_cards_dsl::prelude::*;

/// `{T}: Add {R}.` — Spikefield Cave prints no basic land type, so the
/// intrinsic shortcut (CR 305.6) has nothing to grant it and the ability
/// has to be the card's own.
static CAVE_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])];

card!(
    index = index::SPIKEFIELD_HAZARD,
    oracle_id = "81036c9f-fe0a-45a7-bcd5-0d344f31055a",
    scryfall_id = "a69541db-3f4e-412f-aa8e-dec1e74f74dc",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Spikefield Hazard",
            mana_cost = mana!("{R}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Spikefield Cave",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = CAVE_MANA,
        ),
    ],
    coverage = Coverage::Partial(
        "the rider \"If a permanent dealt damage this way would die this turn, exile it instead\" — the DSL has no replacement effect that redirects a permanent that would die"
    ),
    abilities = &[
        // NOT SUPPORTED: "If a permanent dealt damage this way would die
        // this turn, exile it instead." No `Modifier` and no
        // `ReplacementRule` changes where a permanent goes when it would
        // die, so the damage is dealt and the permanent dies normally.
        spell!(
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget))
        ),
    ],
);
