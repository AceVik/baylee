//! Glissa Sunslayer — {1}{B}{G} — Legendary Creature — Phyrexian Zombie Elf
//! Oracle: First strike, deathtouch
//! Oracle: Whenever Glissa Sunslayer deals combat damage to a player, choose one —
//! Oracle: • You draw a card and lose 1 life.
//! Oracle: • Destroy target enchantment.
//! Oracle: • Remove up to three counters from target permanent.
//! Set: ECC #122 — Lorwyn Eclipsed Commander | Scryfall ID: 682b1b62-338a-49d1-9d4e-2d8ee2e6fc76 | Oracle ID: 8859a692-d107-4de2-9110-4681fc2761c7
// IMPLEMENTED — both combat keywords and every modal combat-damage branch,
// including a distribution of at most three counters over any present kinds.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GLISSA_SUNSLAYER,
    oracle_id = "8859a692-d107-4de2-9110-4681fc2761c7",
    scryfall_id = "682b1b62-338a-49d1-9d4e-2d8ee2e6fc76",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Glissa Sunslayer",
        mana_cost = mana!("{1}{B}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[
            subtypes::creature::PHYREXIAN,
            subtypes::creature::ZOMBIE,
            subtypes::creature::ELF
        ],
        power = Some(3),
        toughness = Some(3),
    ),],
    keywords = KeywordSet::FIRST_STRIKE.union(KeywordSet::DEATHTOUCH),
    coverage = Coverage::Implemented,
    abilities = &[modal_triggered!(
        Trigger::DealsCombatDamageToPlayer(&Filter::This),
        &[
            mode!(&[
                Effect::draw(1),
                Effect::LoseLife {
                    amount: Amount::Fixed(1),
                    target: PlayerRel::You,
                },
            ]),
            mode!(
                &[Effect::destroy(TargetSpec::Object(&Filter::ENCHANTMENT))],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ENCHANTMENT)))
            ),
            mode!(
                &[Effect::RemoveUpToCountersTarget { maximum: 3 }],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any)))
            ),
        ]
    )],
);
