//! Ferocity — {1}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature blocks or becomes blocked, you may put a +1/+1 counter on it.
//! Set: MMQ #245 — Mercadian Masques | Scryfall ID: 4afda489-8397-4ad4-89dc-e8bad92db133 | Oracle ID: 1b1a0f35-3126-4b88-90aa-c56e67850a9d
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "or becomes blocked" on the Aura's own trigger (`Trigger::BecomesBlocked` is source-only and `Trigger::AttachedBlocksOrBecomesBlockedBy` triggers once per blocker rather than once upon becoming blocked).
card!(
    index = index::FEROCITY,
    oracle_id = "1b1a0f35-3126-4b88-90aa-c56e67850a9d",
    scryfall_id = "4afda489-8397-4ad4-89dc-e8bad92db133",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "triggers when enchanted creature blocks; Trigger::BecomesBlocked has no filter parameter for the attached creature",
    ),
    faces = &[face!(
        name = "Ferocity",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        triggered!(
            Trigger::Blocks(&Filter::AttachedToBySource),
            &[Effect::MayDo {
                effects: &[Effect::ChangeAttachedCounters {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                    remove: false,
                }],
            }],
        ),
    ],
);
