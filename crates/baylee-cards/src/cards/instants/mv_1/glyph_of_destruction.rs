//! Glyph of Destruction — {R} — Instant
//! Oracle: Target blocking Wall you control gets +10/+0 until end of combat. Prevent all damage that would be dealt to it this turn. Destroy it at the beginning of the next end step.
//! Set: LEG #150 — Legends | Scryfall ID: 8e9c153c-9224-491b-bc84-8a9f0a83ee5a | Oracle ID: cca68900-2093-4cae-b757-efc7dd807e18
// PARTIAL — the +10/+0 until end of combat and the delayed destroy are built;
// the prevention is off the card.
// NOT SUPPORTED: "Prevent all damage that would be dealt to it this turn." —
// no effect or modifier prevents all noncombat damage to a target for a turn:
// `Modifier::PreventDamageToIt` is combat-only and `Effect::PreventNextDamage`
// is a finite shield.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GLYPH_OF_DESTRUCTION,
    oracle_id = "cca68900-2093-4cae-b757-efc7dd807e18",
    scryfall_id = "8e9c153c-9224-491b-bc84-8a9f0a83ee5a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Glyph of Destruction",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "the prevention half: no effect or modifier prevents all noncombat \
         damage to a target for a turn (PreventDamageToIt is combat-only, \
         PreventNextDamage takes a finite amount)"
    ),
    abilities = &[spell!(
        &[
            Effect::PumpTarget {
                power: Amount::Fixed(10),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfCombat,
            },
            Effect::AtNextEndStep {
                effects: &[Effect::destroy(TargetSpec::EventObject)]
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::And(&[
            Filter::HasSubtype(subtypes::creature::WALL),
            Filter::Blocking,
            Filter::ControlledByYou,
        ]))))
    )],
);
