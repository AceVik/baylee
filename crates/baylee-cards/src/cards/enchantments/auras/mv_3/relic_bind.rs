//! Relic Bind — {2}{U} — Enchantment — Aura
//! Oracle: Enchant artifact an opponent controls
//! Oracle: Whenever enchanted artifact becomes tapped, choose one —
//! Oracle: • This Aura deals 1 damage to target player or planeswalker.
//! Oracle: • Target player gains 1 life.
//! Set: 4ED #97 — Fourth Edition | Scryfall ID: 05eee3dd-e17f-4154-8f8d-29c5421cd89d | Oracle ID: 35b9f327-d1f6-47d1-aefa-6f70f407b4bc
// PARTIAL — the Aura enchants an artifact an opponent controls; the
// becomes-tapped "choose one" is off the card (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "artifact an opponent controls" — the Enchant clause's host.
static OPPONENT_ARTIFACT: Filter = f!(opponents ARTIFACT);

card!(
    index = index::RELIC_BIND,
    oracle_id = "35b9f327-d1f6-47d1-aefa-6f70f407b4bc",
    scryfall_id = "05eee3dd-e17f-4154-8f8d-29c5421cd89d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the becomes-tapped modal trigger's damage mode targets \"target player or \
         planeswalker\", and no TargetSpec spans the seats with a restricted object \
         set: AnyTarget also offers creatures and battles, and OpponentOrObject \
         offers only opponents; dropping one arm of a \"choose one\" would change \
         the other, so the whole trigger is off the card"
    ),
    faces = &[face!(
        name = "Relic Bind",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: "Whenever enchanted artifact becomes tapped, choose one —
    // • This Aura deals 1 damage to target player or planeswalker. • Target
    // player gains 1 life." — the trigger and the life mode are sayable, but
    // the damage mode's target set is not: `TargetSpec::AnyTarget` (CR 115.4)
    // is strictly wider and `TargetSpec::OpponentOrObject(&Filter::PLANESWALKER)`
    // is strictly narrower, and a "choose one" with one arm is a different
    // card.
    abilities = &[spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&OPPONENT_ARTIFACT)
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&OPPONENT_ARTIFACT)))
    ),],
);
