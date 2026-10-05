//! Time Elemental — {2}{U} — Creature — Elemental
//! Oracle: When this creature attacks or blocks, at end of combat, sacrifice it and it deals 5 damage to you.
//! Oracle: {2}{U}{U}, {T}: Return target permanent that isn't enchanted to its owner's hand.
//! Set: ME1 #53 — Masters Edition | Scryfall ID: 720672dc-d75a-44c3-a48d-d2fe3993fbbf | Oracle ID: d89075a3-4413-4796-a055-eef51fddb7f3
// PARTIAL — the attack half of the end-of-combat trigger is built; the
// block half and the activated ability are off the card, see the
// NOT SUPPORTED lines below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TIME_ELEMENTAL,
    oracle_id = "d89075a3-4413-4796-a055-eef51fddb7f3",
    scryfall_id = "720672dc-d75a-44c3-a48d-d2fe3993fbbf",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the block half of the attack-or-block trigger has no one-sided \
         \"blocks\" trigger, and the return's \"isn't enchanted\" target has \
         no Filter, so both are off the card"
    ),
    faces = &[face!(
        name = "Time Elemental",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(0),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: "When this creature attacks or blocks, at end of
    // combat, sacrifice it and it deals 5 damage to you." — the attack half
    // is written (`Trigger::Attacks` with `Effect::AtEndOfCombat`); the
    // block half has no trigger of its own
    // (`Trigger::BlocksOrBecomesBlockedBy` fires on both sides of a block),
    // so it comes off rather than firing twice on a blocked attacker.
    // NOT SUPPORTED: "{2}{U}{U}, {T}: Return target permanent that isn't
    // enchanted to its owner's hand." — no `Filter` names a permanent no
    // Aura is attached to (`Filter::IsAttached` reads the other end of the
    // attachment, on the Aura), so the ability comes off the card instead of
    // bouncing an enchanted permanent too.
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::AtEndOfCombat {
            about: TargetSpec::EventObject,
            effects: &[
                Effect::SacrificeSelf,
                Effect::DealDamage {
                    amount: Amount::Fixed(5),
                    target: TargetSpec::Player(PlayerRel::You),
                },
            ],
        }],
    )],
);
