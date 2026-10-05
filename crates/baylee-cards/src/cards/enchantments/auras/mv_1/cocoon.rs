//! Cocoon — {G} — Enchantment — Aura
//! Oracle: Enchant creature you control
//! Oracle: When this Aura enters, tap enchanted creature and put three pupa counters on this Aura.
//! Oracle: Enchanted creature doesn't untap during your untap step if this Aura has a pupa counter on it.
//! Oracle: At the beginning of your upkeep, remove a pupa counter from this Aura. If you can't, sacrifice it, put a +1/+1 counter on enchanted creature, and that creature gains flying.
//! Set: CHR #59 — Chronicles | Scryfall ID: 897de61e-440f-4eaf-aef8-0dd1c6117288 | Oracle ID: 9e35d450-00af-4f40-a4ee-bc7844ff5628
// PARTIAL — every printed sentence is off the card: the pupa counter has no
// assigned id.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COCOON,
    oracle_id = "9e35d450-00af-4f40-a4ee-bc7844ff5628",
    scryfall_id = "897de61e-440f-4eaf-aef8-0dd1c6117288",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "the pupa counter has no assigned id in baylee_cards_dsl::counters and \
         a card may not write a custom counter id by number, so nothing \
         counted by it can be written"
    ),
    faces = &[face!(
        name = "Cocoon",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: "When this Aura enters, tap enchanted creature and put
    // three pupa counters on this Aura." — the tap is `Effect::TapAll` over
    // `Filter::AttachedToBySource`, but `counters::ASSIGNED` has no pupa id
    // and a bare `CounterKind::Custom(n)` is the collision the registry
    // exists to stop, so the sentence is off whole rather than leaving the
    // creature tapped with nothing to count down.
    // NOT SUPPORTED: "Enchanted creature doesn't untap during your untap step
    // if this Aura has a pupa counter on it." — `Modifier::DoesNotUntap` with
    // `Condition::CountersOnSelf` is the shape, but no pupa counter kind
    // exists to gate on.
    // NOT SUPPORTED: "At the beginning of your upkeep, remove a pupa counter
    // from this Aura. If you can't, sacrifice it, put a +1/+1 counter on
    // enchanted creature, and that creature gains flying." — the removal, the
    // sacrifice branch, the +1/+1 counter and the flying all hinge on the
    // same unassigned pupa counter id.
    abilities = &[spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::YOUR_CREATURE)
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE)))
    )],
);
