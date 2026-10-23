//! Skeleton Scavengers — {2}{B} — Creature — Skeleton
//! Oracle: This creature enters with a +1/+1 counter on it.
//! Oracle: Pay {1} for each +1/+1 counter on this creature: Regenerate this creature. When it regenerates this way, put a +1/+1 counter on it.
//! Set: STH #70 — Stronghold | Scryfall ID: 57af1b3f-661a-4b02-be08-0ec721d7cab8 | Oracle ID: 88782913-bfba-4300-b26c-405e6a45537c
// PARTIAL: scaling the activation cost per +1/+1 counter on this creature and delaying the +1/+1 counter until a regeneration shield is used are not supported; enters with a +1/+1 counter and has "{1}: Regenerate this creature and put a +1/+1 counter on it."

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKELETON_SCAVENGERS,
    oracle_id = "88782913-bfba-4300-b26c-405e6a45537c",
    scryfall_id = "57af1b3f-661a-4b02-be08-0ec721d7cab8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "scaling the activation cost per +1/+1 counter on this creature and delaying the +1/+1 counter until a regeneration shield is used are not supported; enters with a +1/+1 counter and has \"{1}: Regenerate this creature and put a +1/+1 counter on it.\"",
    ),
    faces = &[face!(
        name = "Skeleton Scavengers",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SKELETON],
        power = Some(0),
        toughness = Some(0),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
    ),],
    abilities = &[activated!(
        cost!("{1}"),
        &[
            Effect::Regenerate {
                target: TargetSpec::ThisObject,
            },
            Effect::AddCounterSource {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
        ],
    ),],
);
