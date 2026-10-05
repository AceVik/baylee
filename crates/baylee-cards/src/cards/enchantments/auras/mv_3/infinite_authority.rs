//! Infinite Authority — {W}{W}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature blocks or becomes blocked by a creature with toughness 3 or less, destroy the other creature at end of combat. At the beginning of the next end step, if that creature was destroyed this way, put a +1/+1 counter on the first creature.
//! Set: LEG #22 — Legends | Scryfall ID: dc60077f-d577-4a6c-a78f-697317024c40 | Oracle ID: b4faba1a-23db-4678-9ce6-a7816105f22a
// PARTIAL — the block trigger and its end-of-combat destruction are written;
// the next-end-step counter clause is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INFINITE_AUTHORITY,
    oracle_id = "b4faba1a-23db-4678-9ce6-a7816105f22a",
    scryfall_id = "dc60077f-d577-4a6c-a78f-697317024c40",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "the next-end-step clause is off the card: a delayed trigger remembers \
         one object — AtEndOfCombat's `about` (the other creature) or \
         AtNextEndStep's source (the enchanted creature) — never both, so \
         \"if that creature was destroyed this way\" cannot gate the counter \
         on the first creature"
    ),
    faces = &[face!(
        name = "Infinite Authority",
        mana_cost = mana!("{W}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: "At the beginning of the next end step, if that creature
    // was destroyed this way, put a +1/+1 counter on the first creature." —
    // the destroy is remembered by `Effect::AtEndOfCombat`'s `about`, and
    // `Effect::AtNextEndStep` remembers only its own source (the enchanted
    // creature), so no single delayed trigger can both ask what happened to
    // the other creature and put the counter on the first one.
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        // The trigger belongs to the enchanted creature, so it is granted to
        // it: the other creature of the block pair is the trigger's event
        // object, and `AtEndOfCombat` remembers that exact object for its
        // destroy (CR 603.7c).
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::GrantTriggered {
                trigger: Trigger::BlocksOrBecomesBlockedBy(&Filter::ToughnessAtMost(3)),
                effects: &[Effect::AtEndOfCombat {
                    about: TargetSpec::EventObject,
                    effects: &[Effect::destroy(TargetSpec::EventObject)],
                }],
                target: None,
            }
        ),
    ],
);

// Engine-level coverage belongs in `mechanics_tests::blocks`: grant the
// trigger to a blocker, pair it against a 1/1, and check the 1/1 dies at end
// of combat.
