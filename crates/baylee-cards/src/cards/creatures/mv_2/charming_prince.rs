//! Charming Prince — {1}{W} — Creature — Human Noble
//! Oracle: When this creature enters, choose one —
//! Oracle: • Scry 2.
//! Oracle: • You gain 3 life.
//! Oracle: • Exile another target creature you own. Return it to the battlefield under your control at the beginning of the next end step.
//! Set: FDN #568 — Foundations | Scryfall ID: aa7b47e1-7e32-4f2f-aecf-bac7ca197081 | Oracle ID: c48d844c-3976-4fa5-8e0d-3f0e535e7619
// IMPLEMENTED — all three modes (scry, lifegain, end-step blink).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static SCRY_EFFECTS: &[Effect] = &[Effect::scry(2)];
static LIFE_EFFECTS: &[Effect] = &[Effect::gain_life(3)];
static BLINK_EFFECTS: &[Effect] = &[Effect::ExileAndReturnAtEndStep];
static OTHER_CREATURE_YOU_OWN: Filter =
    Filter::And(&[Filter::Another, Filter::CREATURE, Filter::OwnedByYou]);

card!(
    index = index::CHARMING_PRINCE,
    oracle_id = "c48d844c-3976-4fa5-8e0d-3f0e535e7619",
    scryfall_id = "aa7b47e1-7e32-4f2f-aecf-bac7ca197081",
    faces = &[face!(
        name = "Charming Prince",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[creature::HUMAN, creature::NOBLE],
        power = Some(2),
        toughness = Some(2),
    )],
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    abilities = &[modal_triggered!(
        Trigger::EntersBattlefield(&Filter::This),
        &[
            mode!(SCRY_EFFECTS),
            mode!(LIFE_EFFECTS),
            mode!(
                BLINK_EFFECTS,
                targets = Some(TargetReq::one(TargetSpec::Object(&OTHER_CREATURE_YOU_OWN)))
            ),
        ]
    )],
);
