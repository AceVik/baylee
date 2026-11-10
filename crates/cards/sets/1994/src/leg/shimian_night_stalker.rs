//! Shimian Night Stalker — {3}{B}{B} — Creature — Nightstalker
//! Oracle: {B}, {T}: All damage that would be dealt to you this turn by target attacking creature is dealt to this creature instead.
//! Set: CHR #36 — Chronicles | Scryfall ID: 9caf87f7-36d5-478b-9836-52043833290f | Oracle ID: 09b9e6fd-7a61-4ed4-a121-61b64fbf03f4
// IMPLEMENTED — the targeted attacker's damage to you is redirected to this
// exact permanent for the turn, including noncombat damage.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHIMIAN_NIGHT_STALKER,
    oracle_id = "09b9e6fd-7a61-4ed4-a121-61b64fbf03f4",
    scryfall_id = "9caf87f7-36d5-478b-9836-52043833290f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Shimian Night Stalker",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTSTALKER],
        power = Some(4),
        toughness = Some(4),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{B}", TapSelf),
        &[Effect::RedirectTargetSourceDamageThisTurn {
            source: TargetSpec::Object(&Filter::ATTACKING_CREATURE),
            only_to_you: true,
            to: DamageRedirectTo::ThisObject,
        }],
        target = Some(TargetSpec::Object(&Filter::ATTACKING_CREATURE)),
    )],
);
